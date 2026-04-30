# Pixel Retheme Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace colored triangles with pixel-block airplanes, add particle burst explosions on hit/death, and add procedural sound effects — all generated in code.

**Architecture:** Single custom `Mesh` per airplane built from a pixel grid of quads. `Particle` component drives small square entities that fly outward and fade. Procedural audio waveforms (square wave, noise burst) generated at startup as `AudioSource` assets. No external files needed.

**Tech Stack:** Bevy 0.18 (mesh, color material, audio), bevy_replicon, no new crates.

---

### Task 1: Add Particle component and airplane mesh builder to shared

**Files:**
- Modify: `shared/src/lib.rs` (append new types and function)

- [ ] **Step 1: Add `Particle` component and `build_airplane_mesh()` function**

Add after the existing constants in `shared/src/lib.rs`:

```rust
use bevy::render::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;

#[derive(Component, Clone, Copy, Serialize, Deserialize)]
pub struct Particle {
    pub velocity: (f32, f32),
    pub lifetime: f32,
    pub max_lifetime: f32,
}

pub const PIXEL_SIZE: f32 = 6.0;

const AIRPLANE_PATTERN: &[&[u8]] = &[
    // 12 columns x 10 rows — 0=empty, 1=body
    &[0,0,0,0,1,1,1,1,0,0,0,0],
    &[0,0,0,1,1,1,1,1,1,0,0,0],
    &[0,0,1,1,1,1,1,1,1,1,0,0],
    &[0,1,1,1,1,1,1,1,1,1,1,0],
    &[1,1,1,1,1,1,1,1,1,1,1,1],
    &[0,1,1,1,1,1,1,1,1,1,1,0],
    &[0,0,0,1,1,0,0,1,1,0,0,0],
    &[0,0,0,1,1,0,0,1,1,0,0,0],
    &[0,0,0,0,1,1,1,1,0,0,0,0],
    &[0,0,0,0,1,0,0,1,0,0,0,0],
];

pub fn build_airplane_mesh() -> Mesh {
    let cols = 12;
    let rows = 10;
    let half_w = cols as f32 * PIXEL_SIZE / 2.0;
    let half_h = rows as f32 * PIXEL_SIZE / 2.0;

    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    for (row, row_data) in AIRPLANE_PATTERN.iter().enumerate() {
        for (col, &cell) in row_data.iter().enumerate() {
            if cell == 0 {
                continue;
            }

            let cx = col as f32 * PIXEL_SIZE - half_w + PIXEL_SIZE / 2.0;
            let cy = -(row as f32 * PIXEL_SIZE - half_h + PIXEL_SIZE / 2.0);
            let hs = PIXEL_SIZE / 2.0;

            let base = positions.len() as u32;
            positions.extend_from_slice(&[
                [cx - hs, cy - hs, 0.0],
                [cx + hs, cy - hs, 0.0],
                [cx + hs, cy + hs, 0.0],
                [cx - hs, cy + hs, 0.0],
            ]);
            uvs.extend_from_slice(&[
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [0.0, 1.0],
            ]);
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}
```

- [ ] **Step 2: Build and verify**

Run: `cargo build -p shared 2>&1`
Expected: Compiles successfully (may need API tweaks for Bevy 0.18 exact Mesh API)

If Mesh API differs, fix inline then re-run until green.

- [ ] **Step 3: Commit**

```bash
git add shared/src/lib.rs
git commit -m "feat: add Particle component and airplane mesh builder to shared

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 2: Replace server player mesh with pixel airplane

**Files:**
- Modify: `server/src/render.rs:1-33` (spawn_render function)

- [ ] **Step 1: Rewrite `spawn_render` to use airplane mesh**

Replace the `spawn_render` function in `server/src/render.rs`:

```rust
use shared::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct SpriteReady;

pub fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Transform::default(), GlobalTransform::default()));
}

pub fn spawn_render(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    new_players: Query<(Entity, &PlayerId, &PlayerColor), (With<PlayerId>, Without<SpriteReady>)>,
) {
    for (entity, _player_id, color) in new_players.iter() {
        let airplane = build_airplane_mesh();
        commands.entity(entity).insert((
            SpriteReady,
            Mesh2d(meshes.add(airplane)),
            MeshMaterial2d(materials.add(Color::srgb(color.r, color.g, color.b))),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::VISIBLE,
        ));
    }
}
```

- [ ] **Step 2: Build and verify**

Run: `cargo build -p server 2>&1`
Expected: Compiles successfully.

- [ ] **Step 3: Commit**

```bash
git add server/src/render.rs
git commit -m "feat: replace server player triangle with pixel airplane mesh

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 3: Replace client player mesh with pixel airplane

**Files:**
- Modify: `client/src/render.rs:11-39` (spawn_render function)

- [ ] **Step 1: Rewrite `spawn_render` to use airplane mesh**

Replace the `spawn_render` function in `client/src/render.rs`:

```rust
use crate::LocalClientId;
use shared::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct LocalSprite;

#[derive(Component)]
pub struct LocalPlayer;

pub fn spawn_render(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    local_id: Option<Res<LocalClientId>>,
    new_players: Query<(Entity, &PlayerId, &PlayerColor), (With<PlayerId>, Without<LocalSprite>)>,
) {
    for (entity, player_id, color) in new_players.iter() {
        let airplane = build_airplane_mesh();
        let mut cmd = commands.entity(entity);
        cmd.insert((
            LocalSprite,
            Mesh2d(meshes.add(airplane)),
            MeshMaterial2d(materials.add(Color::srgb(color.r, color.g, color.b))),
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::VISIBLE,
        ));
        if let Some(ref id) = local_id {
            if player_id.0 == id.0 {
                cmd.insert(LocalPlayer);
            }
        }
    }
}
```

- [ ] **Step 2: Build and verify**

Run: `cargo build -p client 2>&1`
Expected: Compiles successfully.

- [ ] **Step 3: Commit**

```bash
git add client/src/render.rs
git commit -m "feat: replace client player triangle with pixel airplane mesh

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 4: Add particle spawn and update systems (server)

**Files:**
- Modify: `server/src/render.rs` (append particle systems after existing code)

- [ ] **Step 1: Add particle systems**

Append after the `apply_bullet_position` function in `server/src/render.rs`:

```rust
pub fn spawn_particle_burst(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    x: f32,
    y: f32,
    color: (f32, f32, f32),
    count: u32,
) {
    let mut rng = rand::rng();
    for _ in 0..count {
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let speed = rng.random_range(80.0..250.0);
        let vx = angle.cos() * speed;
        let vy = angle.sin() * speed;
        let lifetime = rng.random_range(0.3..0.6);

        let mesh = Rectangle2d::new(4.0, 4.0);
        commands.spawn((
            Particle {
                velocity: (vx, vy),
                lifetime,
                max_lifetime: lifetime,
            },
            Mesh2d(meshes.add(mesh)),
            MeshMaterial2d(materials.add(Color::srgb(color.0, color.1, color.2))),
            Transform::from_xyz(x, y, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::VISIBLE,
        ));
    }
}

pub fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut Particle, &mut Transform, &mut Visibility)>,
) {
    let dt = time.delta_secs();
    for (entity, mut particle, mut transform, mut visibility) in particles.iter_mut() {
        particle.lifetime -= dt;
        if particle.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation.x += particle.velocity.0 * dt;
        transform.translation.y += particle.velocity.1 * dt;

        let t = particle.lifetime / particle.max_lifetime;
        if t < 0.2 {
            *visibility = Visibility::Hidden;
        }
    }
}
```

- [ ] **Step 2: Register particle systems in server main.rs**

In `server/src/main.rs`, add `update_particles` to the Update systems chain. Add it after `apply_bullet_position`:

```rust
// In the Update systems chain, add:
update_particles,
```

Full chain should now end with:
```rust
apply_position,
apply_bullet_position,
update_particles,
update_visibility,
update_scoreboard,
```

- [ ] **Step 3: Build and verify**

Run: `cargo build -p server 2>&1`
Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add server/src/render.rs server/src/main.rs
git commit -m "feat: add particle burst and update systems on server

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 5: Add particle spawn and update systems (client)

**Files:**
- Modify: `client/src/render.rs` (append particle systems after existing code)

- [ ] **Step 1: Add particle systems (same as server)**

Append after the `update_visibility` function in `client/src/render.rs`:

```rust
pub fn spawn_particle_burst(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    x: f32,
    y: f32,
    color: (f32, f32, f32),
    count: u32,
) {
    let mut rng = rand::rng();
    for _ in 0..count {
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let speed = rng.random_range(80.0..250.0);
        let vx = angle.cos() * speed;
        let vy = angle.sin() * speed;
        let lifetime = rng.random_range(0.3..0.6);

        let mesh = Rectangle2d::new(4.0, 4.0);
        commands.spawn((
            Particle {
                velocity: (vx, vy),
                lifetime,
                max_lifetime: lifetime,
            },
            Mesh2d(meshes.add(mesh)),
            MeshMaterial2d(materials.add(Color::srgb(color.0, color.1, color.2))),
            Transform::from_xyz(x, y, 0.0),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::VISIBLE,
        ));
    }
}

pub fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut particles: Query<(Entity, &mut Particle, &mut Transform, &mut Visibility)>,
) {
    let dt = time.delta_secs();
    for (entity, mut particle, mut transform, mut visibility) in particles.iter_mut() {
        particle.lifetime -= dt;
        if particle.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }
        transform.translation.x += particle.velocity.0 * dt;
        transform.translation.y += particle.velocity.1 * dt;

        let t = particle.lifetime / particle.max_lifetime;
        if t < 0.2 {
            *visibility = Visibility::Hidden;
        }
    }
}
```

- [ ] **Step 2: Register particle systems in client main.rs**

In `client/src/main.rs`, add `update_particles` to the Update systems chain after `apply_bullet_position`:

```rust
// The chain should now include:
apply_position,
apply_bullet_position,
update_particles,
update_visibility,
update_scoreboard,
```

And add the `rand` dependency to `client/Cargo.toml` (for `rng.random_range`):
```toml
rand = { workspace = true }
```

Note: `spawn_particle_burst` is a utility function called manually (not a system), so it's not registered in the schedule.

- [ ] **Step 3: Build and verify**

Run: `cargo build -p client 2>&1`
Expected: Compiles successfully.

- [ ] **Step 4: Commit**

```bash
git add client/src/render.rs client/src/main.rs client/Cargo.toml
git commit -m "feat: add particle burst and update systems on client

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 6: Trigger particle bursts on combat events (server)

**Files:**
- Modify: `server/src/bullet.rs:125-172` (bullet_player_collision function)
- Modify: `server/src/combat.rs:88-157` (combat_detection function)

- [ ] **Step 1: Add particle burst on bullet-hit in `bullet_player_collision`**

In `server/src/bullet.rs`, modify the collision handler to spawn particles on hit. Add after line 150 (`if health.0 > 0` block, inside the hit detection):

```rust
// After the health decrement, spawn a small hit burst:
// Insert this after line 148 (after health.0 -= 1):
spawn_particle_burst(
    &mut commands,
    &mut meshes,
    &mut materials,
    bullet.x,
    bullet.y,
    (color.r, color.g, color.b),
    8,
);
```

And add the death explosion after inserting the `Dead` component (after line 155):

```rust
// Large explosion on death:
spawn_particle_burst(
    &mut commands,
    &mut meshes,
    &mut materials,
    bullet.x,
    bullet.y,
    (1.0, 0.5, 0.1),
    20,
);
```

Update the function signature to accept `Meshes` and `Materials`:

```rust
pub fn bullet_player_collision(
    mut commands: Commands,
    bullets: Query<(Entity, &Bullet)>,
    mut players: Query<(Entity, &PlayerId, &Position, &Direction, &mut Health), Without<Dead>>,
    mut score_query: Query<(&PlayerId, &mut Score)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
```

And inside the collision loop, extract `color` from the player query. Actually the players query doesn't include `PlayerColor`. Add it:

Change:
```rust
mut players: Query<(Entity, &PlayerId, &Position, &Direction, &mut Health), Without<Dead>>,
```
To:
```rust
mut players: Query<(Entity, &PlayerId, &Position, &Direction, &PlayerColor, &mut Health), Without<Dead>>,
```

And update the destructuring:
```rust
for (player_entity, player_id, player_pos, player_dir, color, mut health) in players.iter_mut() {
```

- [ ] **Step 2: Add particle burst on combat death in `combat_detection`**

In `server/src/combat.rs`, modify the death handling. After line 148 (`commands.entity(*entity).insert((Dead, RespawnTimer...))`), add a particle burst for each killed entity. Need access to `Position`:

Modify the function signature:
```rust
pub fn combat_detection(
    mut commands: Commands,
    players: Query<(Entity, &Position, &Direction), Without<Dead>>,
    mut score_query: Query<&mut Score>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
```

And after each kill insertion, spawn particles:
```rust
// After commands.entity(*entity).insert((Dead, RespawnTimer...)):
// Get position from the killed entity:
if let Ok((_, pos, _)) = players.get(*entity) {
    spawn_particle_burst(
        &mut commands,
        &mut meshes,
        &mut materials,
        pos.x,
        pos.y,
        (1.0, 0.3, 0.1),
        20,
    );
}
```

Wait — `players` is borrowed immutably and `commands` needs mutable borrow. Need to collect positions before the mutable section. Collect positions of killed entities:

```rust
// Before the mutable commands section, collect kill positions:
let kill_positions: Vec<(f32, f32)> = killed.iter()
    .filter_map(|e| players.get(*e).ok().map(|(_, p, _)| (p.x, p.y)))
    .collect();

// Then after inserting Dead components:
for (x, y) in &kill_positions {
    spawn_particle_burst(&mut commands, &mut meshes, &mut materials, *x, *y, (1.0, 0.3, 0.1), 20);
}
```

- [ ] **Step 3: Build and verify**

Run: `cargo build -p server 2>&1`
Expected: Compiles successfully. Fix any borrow checker issues.

- [ ] **Step 4: Commit**

```bash
git add server/src/bullet.rs server/src/combat.rs
git commit -m "feat: trigger particle bursts on bullet hit and player death

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 7: Add procedural sound effects (client)

**Files:**
- Create: `client/src/audio.rs`
- Modify: `client/src/main.rs` (register audio systems)

- [ ] **Step 1: Create the audio module**

Create `client/src/audio.rs`:

```rust
use bevy::audio::AudioSource;
use bevy::prelude::*;
use std::sync::Arc;

#[derive(Resource)]
pub struct GameSounds {
    pub shoot: Handle<AudioSource>,
    pub hit: Handle<AudioSource>,
    pub explosion: Handle<AudioSource>,
}

fn generate_square_wave(freq: f32, duration_secs: f32, sample_rate: u32) -> Vec<i16> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut samples = Vec::with_capacity(num_samples);
    let period = sample_rate as f32 / freq;
    let half_period = period / 2.0;

    for i in 0..num_samples {
        let t = i as f32 % period;
        let amplitude = if t < half_period { 0.3 } else { -0.3 };
        samples.push((amplitude * i16::MAX as f32) as i16);
    }
    samples
}

fn generate_noise(duration_secs: f32, sample_rate: u32) -> Vec<i16> {
    let num_samples = (sample_rate as f32 * duration_secs) as usize;
    let mut rng = rand::rng();
    let mut samples = Vec::with_capacity(num_samples);

    for _ in 0..num_samples {
        let amplitude = rng.random_range(-0.3..0.3);
        samples.push((amplitude * i16::MAX as f32) as i16);
    }
    samples
}

fn generate_shoot_sound() -> Vec<i16> {
    // Descending square wave: 600Hz → 200Hz over 0.08s
    let sample_rate = 44100;
    let duration = 0.08;
    let num_samples = (sample_rate as f32 * duration) as usize;
    let mut samples = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / duration as f32;
        let freq = 600.0 - t * 400.0;
        let period = sample_rate as f32 / freq;
        let phase = (i as f32) % period;
        let amplitude = if phase < period / 2.0 { 0.2 } else { -0.2 };
        let decay = 1.0 - t;
        samples.push((amplitude * decay * i16::MAX as f32) as i16);
    }
    samples
}

fn generate_hit_sound() -> Vec<i16> {
    // Short noise burst, 0.1s
    let sample_rate = 44100;
    let num_samples = (sample_rate as f32 * 0.1) as usize;
    let mut rng = rand::rng();
    let mut samples = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / num_samples as f32;
        let amplitude = rng.random_range(-0.25..0.25);
        let decay = 1.0 - t;
        samples.push((amplitude * decay * i16::MAX as f32) as i16);
    }
    samples
}

fn generate_explosion_sound() -> Vec<i16> {
    // Noise burst + low square wave, 0.3s
    let sample_rate = 44100;
    let num_samples = (sample_rate as f32 * 0.3) as usize;
    let mut rng = rand::rng();
    let mut samples = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let t = i as f32 / num_samples as f32;
        let decay = (1.0 - t).max(0.0);

        let noise = rng.random_range(-0.3..0.3);
        let period = sample_rate as f32 / 80.0;
        let phase = (i as f32) % period;
        let square = if phase < period / 2.0 { 0.15 } else { -0.15 };

        let amplitude = (noise * 0.6 + square * 0.4) * decay;
        samples.push((amplitude * i16::MAX as f32) as i16);
    }
    samples
}

pub fn generate_game_sounds(mut commands: Commands, mut audio_sources: ResMut<Assets<AudioSource>>) {
    fn to_audio_source(samples: Vec<i16>) -> AudioSource {
        let arc: Arc<[i16]> = samples.into();
        AudioSource {
            samples: arc,
            sample_rate: 44100,
            channels: 1,
        }
    }

    let shoot_handle = audio_sources.add(to_audio_source(generate_shoot_sound()));
    let hit_handle = audio_sources.add(to_audio_source(generate_hit_sound()));
    let explosion_handle = audio_sources.add(to_audio_source(generate_explosion_sound()));

    commands.insert_resource(GameSounds {
        shoot: shoot_handle,
        hit: hit_handle,
        explosion: explosion_handle,
    });

    info!("Procedural game sounds generated");
}
```

Note: The `AudioSource` struct field access may differ in Bevy 0.18. If fields are private, use:
```rust
fn to_audio_source(samples: Vec<i16>) -> AudioSource {
    // Alternative: use bevy's audio source creation API
    AudioSource::new()
    // ... fill in based on available API
}
```
During implementation, check the Bevy 0.18 audio API and adjust accordingly.

- [ ] **Step 2: Register audio module in client main.rs**

In `client/src/main.rs`:

Add module declaration at top:
```rust
mod audio;
```

Add `use audio::{GameSounds, generate_game_sounds};` to imports.

Add `generate_game_sounds` to Startup systems (after `setup_scoreboard`):
```rust
app.add_systems(Startup, (setup_camera, setup_scoreboard, generate_game_sounds));
```

- [ ] **Step 3: Add sound playback triggers**

Add a system in `client/src/audio.rs` that watches for combat events and plays sounds:

```rust
pub fn play_combat_sounds(
    mut commands: Commands,
    sounds: Res<GameSounds>,
    players: Query<&Health, (With<PlayerId>, Changed<Health>)>,
    new_dead: Query<Entity, Added<Dead>>,
    new_bullets: Query<Entity, Added<Bullet>>,
) {
    // Shoot sound on new bullets
    if new_bullets.iter().next().is_some() {
        commands.spawn((
            AudioPlayer::new(sounds.shoot.clone()),
            PlaybackSettings::DESPAWN,
        ));
    }

    // Hit sound on health change
    for health in players.iter() {
        if health.0 > 0 {
            commands.spawn((
                AudioPlayer::new(sounds.hit.clone()),
                PlaybackSettings::DESPAWN,
            ));
        }
    }

    // Explosion sound on death
    if new_dead.iter().next().is_some() {
        commands.spawn((
            AudioPlayer::new(sounds.explosion.clone()),
            PlaybackSettings::DESPAWN,
        ));
    }
}
```

Register this system in client main.rs Update chain:
```rust
// In the InGame update chain, add:
update_particles,
play_combat_sounds,
update_visibility,
```

- [ ] **Step 4: Build and verify**

Run: `cargo build -p client 2>&1`
Expected: May need API adjustments for Bevy 0.18 audio. Fix inline until compiles.

Common adjustments needed:
- `AudioPlayer::new()` → might be different in 0.18
- `AudioSource` fields might be private → check docs
- `PlaybackSettings::DESPAWN` → might be `PlaybackSettings::ONCE`

- [ ] **Step 5: Commit**

```bash
git add client/src/audio.rs client/src/main.rs
git commit -m "feat: add procedural sound effects (shoot, hit, explosion)

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

### Task 8: Integration test — build and verify full workspace

- [ ] **Step 1: Build entire workspace**

Run: `cargo build --workspace 2>&1`
Expected: All 5 crates compile with zero errors.

- [ ] **Step 2: Review all changes**

Run: `git diff main --stat`
Expected: See all modified and new files.

- [ ] **Step 3: Run the game**

Manual test:
1. Start platform: `cargo run -p platform`
2. Start server: `cargo run -p server`
3. Start client: `cargo run -p client`
4. Login and verify: airplane shape visible, particles on hit/death, sounds play

- [ ] **Step 4: Commit any final fixes**

```bash
git add -A
git commit -m "chore: final integration fixes for pixel retheme

Co-Authored-By: Claude Opus 4.7 <noreply@anthropic.com>"
```

---

## Implementation Order

```
Task 1 (shared types) → Task 2 (server airplane) → Task 3 (client airplane)
                                                    ↘
Task 1 (shared types) → Task 4 (server particles) → Task 5 (client particles)
                                                    ↘
                        Task 6 (combat triggers)    → Task 7 (audio) → Task 8 (integration)
```

Tasks 2/3 can run in parallel after Task 1. Tasks 4/5 can run in parallel after Task 1. Task 6 depends on Task 4. Task 7 is independent of Task 6 (can run in parallel).
