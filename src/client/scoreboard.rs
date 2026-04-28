use bevy::prelude::*;
use crate::shared::{PlayerId, Score, PlayerColor};

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
