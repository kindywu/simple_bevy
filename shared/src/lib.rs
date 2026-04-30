use bevy::prelude::*;
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
