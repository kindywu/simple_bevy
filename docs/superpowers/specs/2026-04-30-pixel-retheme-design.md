# Pixel Retheme — Design Spec

Date: 2026-04-30

## Summary

Retheme the multiplayer game from colored triangles to pixel-retro airplanes with procedural explosion effects and sound effects. Everything is generated in code — zero external asset files.

## Decisions

| Aspect | Choice |
|--------|--------|
| Visual style | Pixel retro |
| Asset approach | 100% procedural (no image/audio files) |
| Airplane style | Pixel-block airplane (Rectangle mesh composite) |
| Explosion style | Particle burst (15–25 square particles flying outward) |
| Sound effects | Procedural waveform synthesis (square wave, noise burst) |
| Background music | Skipped for now |

## Architecture

No new crates. Everything built on Bevy 0.18 built-in primitives (mesh, color material, audio).

### 1. Pixel Airplane

Replace `Triangle2d` player mesh with a composite of small `Rectangle2d` / `Quad` meshes arranged in a pixel-art airplane shape. Defined as a grid pattern in code.

- **Approach**: Spawn child entities for each "pixel block" under the player entity, or generate a single custom `Mesh` from vertex data
- **Recommended**: Generate a single custom `Mesh` — fewer entities, simpler transform management
- **Engine flame**: 1–2 small rectangles at the rear that oscillate in size/color (yellow→orange→red) via a simple timer

Player entity still has `Position`, `Direction`, `PlayerColor` — the color tints the airplane body.

### 2. Particle Explosion System

Generic particle system for explosions and hit sparks.

**Components** (in `shared/src/lib.rs`):
```rust
struct Particle {
    velocity: Vec2,
    lifetime: f32,
    max_lifetime: f32,
    start_color: (f32, f32, f32),
    end_color: (f32, f32, f32),
}
```

**Spawning**: On death or bullet hit, spawn 15–25 small square entities at the event position, each with random velocity and ~0.5s lifetime.

**Update system**: Decrease lifetime, interpolate color, fade out on death, despawn when lifetime reaches 0.

**Triggers**:
- Player killed → large particle burst (20 particles, orange/red)
- Bullet hits player → small particle burst (8 particles, yellow/white)
- Bullet expires / hits boundary → nothing (too noisy)

### 3. Procedural Sound Effects

Generate PCM audio data at startup and store as `AudioSource` assets. Play on events.

**Sound types**:
| Sound | Waveform | Duration | Character |
|-------|----------|----------|-----------|
| Shoot | Square wave, descending pitch | 0.08s | "pew" laser shot |
| Hit | White noise burst | 0.1s | Impact spark |
| Explosion | White noise + low square wave | 0.3s | Low boom |

Generated once at startup via a system that builds `Vec<i16>` PCM buffers, creates `AudioSource` from them, and stores handles in a resource.

**Triggers**: Server observes add/insert of `Dead` component (explosion sound), bullet-player collision (hit sound), bullet spawn (shoot sound). Sound events replicated or triggered locally on client.

Actually: sounds should be **client-side**. The server triggers spawn of replicated marker entities / events, and the client plays sounds in response to those visual events. Or simpler: client plays sounds based on local observations (bullet spawn, entity death).

### 4. File Changes

| File | Changes |
|------|---------|
| `shared/src/lib.rs` | Add `Particle` component, sound event types |
| `server/src/render.rs` | Airplane mesh builder, particle spawn/update systems |
| `client/src/render.rs` | Mirror airplane + particle systems, add sound generation/playback |
| `server/src/bullet.rs` | Emit hit events for explosion spawns |
| `server/src/combat.rs` | Emit death explosion events |
| `client/src/main.rs` | Register new systems, add `AudioPlugin` if not already present |

### 5. Systems Flow

```
Server:
  bullet hit detected → spawn Particle entities (replicated to clients)
  player killed → spawn large Particle burst (replicated to clients)

Client:
  new Bullet observed → play "shoot" sound
  Particle entities spawned → play "hit" / "explosion" sound based on count
  Particle system: update lifetimes, interpolate colors, despawn on expiry

Both:
  spawn_render → build airplane mesh instead of triangle
  apply_position → unchanged (works with any mesh)
```

### 6. Color Scheme

Players retain the golden-angle color generation. Each player's airplane is tinted with their assigned color. Particles use their owner's color for the initial burst, transitioning to dark red/transparent.

### 7. What Does NOT Change

- Networking (bevy_replicon, renet) — unchanged
- Combat logic (triangle collision → becomes hitbox-based or stays as geometric detection)
- Login flow, platform auth, scoreboard
- Movement, shooting mechanics, HP system

### 8. Open Question

Should airplane hitbox stay as triangle-tip logic or change to a rectangle/AABB? Keep triangle-tip for now — simpler, and the airplane mesh orientation still points forward.
