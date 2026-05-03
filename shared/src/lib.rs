use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

pub const PORT: u16 = 5000;
pub const PROTOCOL_ID: u64 = 123456;
pub const PLATFORM_PORT: u16 = 3001;
pub const PLATFORM_API_KEY: &str = "super-secret-platform-api-key";
pub const PLATFORM_HOST: &str = "127.0.0.1";

#[derive(Component, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Component, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct PlayerId(pub u64);

#[derive(Component, Clone, Copy, Serialize, Deserialize)]
pub struct PlayerColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

#[derive(Message, Clone, Serialize, Deserialize)]
pub struct MoveInput {
    pub dx: f32,
    pub dy: f32,
}

#[derive(Component, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Direction {
    pub angle: f32,
}

#[derive(Component, Clone, Copy, Serialize, Deserialize, Default, Debug)]
pub struct Score(pub u32);

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug)]
pub struct Dead;

#[derive(Resource, Default)]
pub struct PlayerCount(pub u32);

#[derive(Component, Clone, Serialize, Deserialize)]
pub struct PlayerName(pub String);

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AuthCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AuthResponse {
    pub success: bool,
    pub username: String,
    pub message: String,
    pub token: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RenewRequest {
    pub token: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RenewResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Message, Clone, Serialize, Deserialize)]
pub struct ShootInput(pub u8);

#[derive(Component, Clone, Copy, Serialize, Deserialize)]
pub struct Health(pub u8);

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug)]
pub struct Bullet {
    pub owner: u64,
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub speed: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

pub const MAX_HP: u8 = 3;
pub const MAX_BULLETS_PER_PLAYER: usize = 5;
pub const SHOOT_COOLDOWN_SECS: f32 = 0.3;
pub const BULLET_SPEED: f32 = 500.0;
pub const BULLET_LIFETIME_SECS: f32 = 2.0;

#[derive(Component)]
pub struct ScoreboardWidget;

#[derive(Component)]
pub struct ScoreboardEntry;

pub fn register_replicon_setup(app: &mut App) {
    app.add_plugins((RepliconPlugins, bevy_replicon_renet::RepliconRenetPlugins));

    app.replicate::<Position>();
    app.replicate::<Direction>();
    app.replicate::<PlayerId>();
    app.replicate::<PlayerColor>();
    app.replicate::<Score>();
    app.replicate::<Dead>();
    app.replicate::<PlayerName>();
    app.replicate::<Health>();
    app.replicate::<Bullet>();

    app.add_client_message::<MoveInput>(Channel::Ordered);
    app.add_client_message::<ShootInput>(Channel::Ordered);
    app.init_resource::<PlayerCount>();
}

pub fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Transform::default(), GlobalTransform::default()));
}

pub fn apply_position(mut entities: Query<(&Position, &Direction, &mut Transform)>) {
    for (pos, dir, mut transform) in entities.iter_mut() {
        transform.translation = Vec3::new(pos.x, pos.y, 0.0);
        transform.rotation = Quat::from_rotation_z(dir.angle);
    }
}

pub fn apply_bullet_position(mut bullets: Query<(&Bullet, &mut Transform)>) {
    for (bullet, mut transform) in bullets.iter_mut() {
        transform.translation = Vec3::new(bullet.x, bullet.y, 0.0);
        transform.rotation = Quat::from_rotation_z(bullet.angle);
    }
}

pub fn update_visibility(
    mut dead: Query<&mut Visibility, With<Dead>>,
    mut alive: Query<&mut Visibility, (With<PlayerId>, Without<Dead>)>,
) {
    for mut vis in dead.iter_mut() {
        if *vis != Visibility::Hidden {
            *vis = Visibility::Hidden;
        }
    }
    for mut vis in alive.iter_mut() {
        if *vis != Visibility::Inherited {
            *vis = Visibility::Inherited;
        }
    }
}
