use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const PORT: u16 = 5000;
pub const PROTOCOL_ID: u64 = 123456;

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
