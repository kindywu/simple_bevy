# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

This is a Cargo workspace with 5 crates: `platform`, `shared`, `server`, `client`, `lab`.

```bash
# Build everything
cargo build --workspace

# Platform auth service (start this first)
cargo run -p platform

# Core multiplayer game
cargo run -p server    # Start server (reads ../.env for PLATFORM_API_KEY)
cargo run -p client    # Start client (login UI → game)

# Secure client release (excludes server code from binary)
cargo build -p client --release

# Examples (in lab/examples/)
cargo run -p lab
cargo run -p lab --example finance
cargo run -p lab --example single -- server
cargo run -p lab --example single -- client
cargo run -p lab --example simple_finance
```

There are no tests in this project.

## Architecture

### Platform (`platform/`)

Independent crate — axum HTTPS server on `127.0.0.1:3001` using `axum-server` with rustls TLS (mkcert certificates). Stores player credentials in `players.json` (SHA-256 hashed passwords) and valid server API keys in `api_keys.json`. The game server calls `POST /api/auth` over HTTPS with an `Authorization: Bearer <key>` header to validate players before spawning them. Default users: kindy, ananda, martin, amy (password = username).

### Shared (`shared/`)

Library crate with all shared ECS components, messages, resources, and constants (`src/lib.rs`). Depends on `bevy`, `bevy_replicon`, and `serde`. Both `server` and `client` crates depend on this.

### Server (`server/`)

Binary crate — the game server. Entry point is `src/main.rs`, which contains the `run()` function (app setup, system registration, observer wiring) and `main()` (.env loading). Submodules: `auth` (platform API calls over HTTPS with custom rustls TLS agent, session renewal), `bullet` (shooting, movement, collision), `combat` (triangle tip-vs-body detection, respawn), `render` (mesh spawning, transform sync), `scoreboard` (centered Chinese UI).

### Client (`client/`)

Binary crate — the game client. Entry point is `src/main.rs`, with `run()` (app setup, state-based system gating) and `main()`. Submodules: `login` (two-step username/password UI, renet connection), `render` (mesh spawning with `LocalPlayer` marker), `scoreboard` (top-right simple text).

**Authentication flow**:
1. Client starts in `GameState::Login` — shows bevy_ui login screen (two-step: username, then password)
2. User submits → client creates renet connection with credentials serialized into `ClientAuthentication::Unsecure.user_data` (256-byte field)
3. Server's `server_on_connect` extracts credentials via `NetcodeServerTransport::user_data()`, calls platform `/api/auth` over HTTPS to validate
4. On success: spawns player entity with `PlayerName`; on failure: calls `server.disconnect()` to reject client
5. Server reads `PLATFORM_API_KEY` from `.env` file at project root

**Networking model**: Server-authoritative via `bevy_replicon`. Components marked `Replicated` (spawned server-side) auto-sync to all clients. Client sends `MoveInput` messages (not replicated — uses `add_client_message`). `Position` and `Direction` are server-authoritative and replicated back.

**Key components** (all in `shared/src/lib.rs`):
- `Position` (x, y) — replicated, server-authoritative
- `Direction` (angle) — replicated, server-authoritative, rotation around Z
- `PlayerId(u64)` — replicated, set from client entity bits
- `PlayerName(String)` — replicated, set from platform validation
- `PlayerColor` (r, g, b) — replicated, derived from `PlayerCount` via golden angle
- `PLATFORM_HOST` / `PLATFORM_PORT` — platform connection constants
- `MoveInput` (dx, dy) — message type, client→server
- `LocalSprite` / `LocalPlayer` — marker components, client-side only
- `AuthCredentials` / `AuthResponse` / `RenewRequest` / `RenewResponse` — auth serialization types, shared between server and platform

**Key systems**:
- `spawn_render` — creates `Triangle2d` mesh + `LocalPlayer` marker for local player
- `apply_position` — syncs `Position` (translation) + `Direction` (Z rotation) to `Transform`
- `server_handle_input` — reads `MoveInput` messages, updates Position and Direction
- `client_send_input` — reads keyboard, normalizes input, sends `MoveInput`, updates local Direction
- `check_connection` — 5-second timeout or server disconnect → transitions back to `GameState::Login`
- `server_on_connect` — validates credentials via platform API over HTTPS (custom rustls TLS agent trusting mkcert CA), prevents duplicate logins, finds safe spawn position, spawns player on success
- `server_on_disconnect` — cleans up `OnlinePlayers` mappings and despawns the player's entity
- `renew_sessions_system` — periodically renews all online player session tokens, disconnects on failure
- Login UI systems (`handle_login_input`, `render_login_text`, `handle_connect`) — only run in `GameState::Login`

**Direction/rotation logic**: Angle is set to `atan2(dy, dx) - FRAC_PI_2` so the triangle's tip points in the movement direction. On the client, local Direction is updated immediately for responsive feel; on the server, Direction is only updated when input is non-zero.

**Rendering**: Uses `Triangle2d` mesh (not sprites). Each player entity gets `Mesh2d` + `ColorMaterial`. `MeshMaterial2d` requires `bevy` 0.18's color material system. Both server and client use `bevy_ui_widgets` for interactive UI buttons in scoreboard and login screen.

### Lab (`lab/`)

Library crate hosting standalone examples and demos. Not depended on by any other crate — exists purely to run examples via `cargo run -p lab --example`.

### Finance Example (`lab/examples/finance.rs`)

A separate binary: Bevy ECS-based trading engine with axum REST API + sled persistence. Order matching runs as a Bevy system. REST endpoints: `GET /trades`, `GET /accounts`, `GET /orders`, `POST /orders`. HTTP test file at `rest/test.rest`.

### Other Examples

- `lab/examples/single.rs` — same multiplayer game but everything in one file (no modules)
- `lab/examples/simple_finance.rs` — ECS finance demo without networking/persistence
- `lab/examples/server.rs` / `lab/examples/client.rs` — minimal UDP socket tests

## Key Dependencies

| Crate                                | Usage                                              |
| ------------------------------------ | -------------------------------------------------- |
| `bevy` 0.18                          | Game engine (all examples)                         |
| `bevy_replicon`                      | Network replication (core game, single example)    |
| `bevy_replicon_renet` / `bevy_renet` | renet transport layer                              |
| `bevy_ui_widgets`                    | UI buttons and interaction (login, scoreboard)     |
| `axum` + `axum-server` + `tokio`    | HTTPS REST API (platform, finance example)         |
| `sled`                               | Embedded DB for persistence (finance example only) |
| `serde` + `bincode`                  | Serialization (all binaries)                       |
| `serde_json`                         | JSON serialization (credentials, platform API)     |
| `ureq` + `rustls`                    | HTTPS client with custom TLS (server→platform)     |
| `sha2` + `hex`                       | Password hashing (platform only)                   |

## Code Patterns

- **World-based startup**: Server startup function takes `&mut World` (not `Commands`), because it inserts resources into the world before the schedule runs. Client startup uses systems and `Commands`.
- **ClientId mapping**: `client_id_to_u64()` converts `ClientId::Client(entity)` to `entity.to_bits()` for matching against `PlayerId.0`. This is how server maps incoming messages to the correct player entity.
- **Golden angle color generation**: `hue = (count * 137.508) % 360` produces well-distributed distinct hues for successive players.
- **State-based UI**: Client uses `GameState` enum (`Login`, `InGame`) to gate systems with `run_if(in_state(...))`. Login UI is two-step: username prompt, then password prompt.
- **Credentials in user_data**: Client serializes `AuthCredentials` as JSON into the 256-byte `user_data` field of `ClientAuthentication::Unsecure`. Server extracts it via `NetcodeServerTransport::user_data(client_id)` and validates over HTTPS using a custom rustls TLS agent.
- **Editions**: Uses Rust edition 2024 (`Cargo.toml`).
- **Workspace dependencies**: Shared crate versions (`bevy`, `bevy_renet`, `bevy_replicon`, `bevy_replicon_renet`, `serde`, `serde_json`, `rand`) are centralized in root `Cargo.toml` under `[workspace.dependencies]`. Member crates reference them with `workspace = true`. Platform-only deps (`axum`, `axum-server`, `tokio`, `sha2`, `hex`) stay in platform's own `Cargo.toml`. Server-only deps (`ureq`, `rustls`, `rustls-pemfile`, `rustls-native-certs`) stay in server's `Cargo.toml`.
