use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const PORT: u16 = 5000;
pub const MOVE_SPEED: f32 = 300.0;
pub const PROTOCOL_ID: u64 = 123456;

#[derive(Component, Clone, Copy, Serialize, Deserialize)]
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

#[derive(Component)]
pub struct LocalSprite;

#[derive(Resource, Default)]
pub struct PlayerCount(pub u32);

#[derive(Resource)]
pub struct ConnectTimer(pub Timer);

#[derive(Resource, Default)]
pub struct ConnectionState {
    pub printed_connected: bool,
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h = h / 60.0;
    let i = h.floor() as u32 % 6;
    let f = h - h.floor();
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    }
}

pub fn client_spawn_render(
    mut commands: Commands,
    new_players: Query<(Entity, &PlayerColor), (With<PlayerId>, Without<LocalSprite>)>,
) {
    for (entity, color) in new_players.iter() {
        commands.entity(entity).insert((
            LocalSprite,
            Sprite {
                color: Color::srgb(color.r, color.g, color.b),
                custom_size: Some(Vec2::splat(40.0)),
                ..default()
            },
            Transform::default(),
            GlobalTransform::default(),
            Visibility::default(),
            InheritedVisibility::VISIBLE,
        ));
    }
}

pub fn client_apply_position(mut players: Query<(&Position, &mut Transform), With<PlayerId>>) {
    for (pos, mut transform) in players.iter_mut() {
        transform.translation = Vec3::new(pos.x, pos.y, 0.0);
    }
}
