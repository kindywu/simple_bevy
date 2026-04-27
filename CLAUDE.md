# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
# Core multiplayer game
cargo run -- server          # Start server (listens on UDP :5000)
cargo run -- client          # Start client (connects to localhost:5000)

# Finance trading engine (Bevy ECS + axum REST + sled DB)
cargo run --example finance

# Standalone single-file multiplayer (no external modules)
cargo run --example single -- server
cargo run --example single -- client

# Simplified finance demo
cargo run --example simple_finance
```

There are no tests in this project.

## Architecture

### Core Game (`src/`)

Single binary, mode selected via CLI arg (`server` | `client`). All ECS components and shared systems live in `src/shared.rs`. Server systems in `src/server.rs`, client systems in `src/client.rs`. `src/main.rs` registers plugins and conditionally adds server-only or client-only systems based on the CLI arg.

**Networking model**: Server-authoritative via `bevy_replicon`. Components marked `Replicated` (spawned server-side) auto-sync to all clients. Client sends `MoveInput` messages (not replicated — uses `add_client_message`). `Position` and `Direction` are server-authoritative and replicated back.

**Key components** (all in `shared.rs`):
- `Position` (x, y) — replicated, server-authoritative
- `Direction` (angle) — replicated, server-authoritative, rotation around Z
- `PlayerId(u64)` — replicated, set from client entity bits
- `PlayerColor` (r, g, b) — replicated, derived from `PlayerCount` via golden angle
- `MoveInput` (dx, dy) — message type, client→server
- `LocalSprite` / `LocalPlayer` — marker components, client-side only

**Key systems**:
- `spawn_render` — creates `Triangle2d` mesh + `LocalPlayer` marker for local player
- `apply_position` — syncs `Position` (translation) + `Direction` (Z rotation) to `Transform`
- `server_handle_input` — reads `MoveInput` messages, updates Position and Direction
- `client_send_input` — reads keyboard, normalizes input, sends `MoveInput`, updates local Direction
- `check_connection` — 5-second timeout, panics if not connected

**Direction/rotation logic**: Angle is set to `atan2(dy, dx) - FRAC_PI_2` so the triangle's tip points in the movement direction. On the client, local Direction is updated immediately for responsive feel; on the server, Direction is only updated when input is non-zero.

**Rendering**: Uses `Triangle2d` mesh (not sprites). Each player entity gets `Mesh2d` + `ColorMaterial`. `MeshMaterial2d` requires `bevy` 0.18's color material system.

### Finance Example (`examples/finance.rs`)

A separate binary: Bevy ECS-based trading engine with axum REST API + sled persistence. Order matching runs as a Bevy system. REST endpoints: `GET /trades`, `GET /accounts`, `GET /orders`, `POST /orders`. HTTP test file at `rest/test.rest`.

### Other Examples

- `examples/single.rs` — same multiplayer game but everything in one file (no `src/` modules)
- `examples/simple_finance.rs` — ECS finance demo without networking/persistence
- `examples/server.rs` / `examples/client.rs` — minimal UDP socket tests

## Key Dependencies

| Crate | Usage |
|-------|-------|
| `bevy` 0.18 | Game engine (all examples) |
| `bevy_replicon` | Network replication (core game, single example) |
| `bevy_replicon_renet` / `bevy_renet` | renet transport layer |
| `axum` + `tokio` | REST API (finance example only) |
| `sled` | Embedded DB for persistence (finance example only) |
| `serde` + `bincode` | Serialization (all binaries) |

## Code Patterns

- **World-based startup**: Server/client startup functions take `&mut World` (not `Commands`), because they insert resources into the world before the schedule runs. Systems use `Commands` normally.
- **ClientId mapping**: `client_id_to_u64()` converts `ClientId::Client(entity)` to `entity.to_bits()` for matching against `PlayerId.0`. This is how server maps incoming messages to the correct player entity.
- **Golden angle color generation**: `hue = (count * 137.508) % 360` produces well-distributed distinct hues for successive players.
- **Editions**: Uses Rust edition 2024 (`Cargo.toml`).
