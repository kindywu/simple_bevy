use bevy::prelude::*;
use crate::shared::{PlayerId, Score, PlayerColor, PlayerName};

#[derive(Component)]
pub struct ScoreboardRoot;

pub fn setup_scoreboard(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/msyh.ttc");
    commands.spawn((
        Text::new(""),
        TextFont { font, font_size: 20.0, ..default() },
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
    players: Query<(&PlayerId, &Score, &PlayerColor, &PlayerName)>,
    asset_server: Res<AssetServer>,
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
        for (_player_id, score, _color, name) in &player_data {
            text.push_str(&format!("\n{}: {}", name.0, score.0));
        }
    }

    let entry = commands
        .spawn((
            TextSpan(text.into()),
            TextFont {
                font: asset_server.load("fonts/msyh.ttc"),
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id();
    commands.entity(entry).set_parent_in_place(root);
    prev_entries.push(entry);
}
