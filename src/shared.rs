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

#[derive(Component)]
pub struct LocalSprite;

#[derive(Component)]
pub struct LocalPlayer;

#[derive(Component, Clone, Copy, Serialize, Deserialize, Default)]
pub struct Direction {
    pub angle: f32,
}

#[derive(Component, Clone, Copy, Serialize, Deserialize, Default, Debug)]
pub struct Score(pub u32);

#[derive(Component, Clone, Copy, Serialize, Deserialize, Debug)]
pub struct Dead;

#[derive(Resource)]
pub struct LocalClientId(pub u64);

#[derive(Resource, Default)]
pub struct PlayerCount(#[allow(dead_code)] pub u32);


pub fn spawn_render(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    local_id: Option<Res<LocalClientId>>,
    new_players: Query<(Entity, &PlayerId, &PlayerColor), (With<PlayerId>, Without<LocalSprite>)>,
) {
    for (entity, player_id, color) in new_players.iter() {
        let mesh = Triangle2d::new(
            Vec2::new(0.0, 20.0),
            Vec2::new(-15.0, -20.0),
            Vec2::new(15.0, -20.0),
        );
        let mut cmd = commands.entity(entity);
        cmd.insert((
            LocalSprite,
            Mesh2d(meshes.add(mesh)),
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

pub fn apply_position(
    mut players: Query<(&Position, &Direction, &mut Transform), With<PlayerId>>,
) {
    for (pos, dir, mut transform) in players.iter_mut() {
        transform.translation = Vec3::new(pos.x, pos.y, 0.0);
        transform.rotation = Quat::from_rotation_z(dir.angle);
    }
}

pub fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Transform::default(), GlobalTransform::default()));
}

#[derive(Component)]
pub struct ScoreboardRoot;

pub fn setup_scoreboard(mut commands: Commands) {
    commands.spawn((
        Text::new(""),
        TextFont::from_font_size(20.0),
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(15.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexEnd,
            row_gap: Val::Px(2.0),
            ..default()
        },
        GlobalZIndex(10),
        ScoreboardRoot,
    ));
}

pub fn update_scoreboard(
    mut commands: Commands,
    scoreboard: Query<Entity, With<ScoreboardRoot>>,
    mut prev_entries: Local<Vec<Entity>>,
    players: Query<(&PlayerId, &Score, &PlayerColor)>,
) {
    let Ok(root) = scoreboard.single() else {
        return;
    };

    for &entity in prev_entries.iter() {
        commands.entity(entity).despawn();
    }
    prev_entries.clear();

    let mut player_data: Vec<_> = players.iter().collect();
    player_data.sort_unstable_by(|a, b| b.1 .0.cmp(&a.1 .0));

    let mut text = "=== Scores ===".to_string();
    if player_data.is_empty() {
        text.push_str("\nWaiting...");
    } else {
        for (player_id, score, _color) in &player_data {
            let short_id = player_id.0 % 1000;
            text.push_str(&format!("\nP{short_id}: {}", score.0));
        }
    }

    let entry = commands
        .spawn((
            TextSpan(text.into()),
            TextFont::from_font_size(18.0),
            TextColor(Color::WHITE),
        ))
        .id();
    commands.entity(entry).set_parent_in_place(root);
    prev_entries.push(entry);
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
