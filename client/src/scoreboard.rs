use bevy::prelude::*;
use bevy_ui_widgets::{Button, observe};
use shared::{PlayerColor, PlayerId, PlayerName, Score, ScoreboardEntry, ScoreboardWidget};

pub fn setup_scoreboard(mut commands: Commands) {
    commands.spawn((
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
        ScoreboardWidget,
    ));
}

pub fn update_scoreboard(
    mut commands: Commands,
    scoreboard: Query<Entity, With<ScoreboardWidget>>,
    mut prev_entries: Local<Vec<Entity>>,
    players: Query<(&PlayerId, &Score, &PlayerColor, &PlayerName)>,
) {
    let Ok(root) = scoreboard.single() else {
        return;
    };

    for &entity in prev_entries.iter() {
        commands.entity(entity).despawn();
    }
    prev_entries.clear();

    let mut player_data: Vec<_> = players.iter().collect();
    player_data.sort_unstable_by(|a, b| b.1.0.cmp(&a.1.0));

    let text = if player_data.is_empty() {
        "=== Scores ===\nWaiting...".to_string()
    } else {
        let mut lines = "=== Scores ===".to_string();
        for (_player_id, score, _color, name) in &player_data {
            lines.push_str(&format!("\n{}: {}", name.0, score.0));
        }
        lines
    };

    let entry = commands
        .spawn((
            ScoreboardEntry,
            Button,
            Node {
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            Text::new(text),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::WHITE),
            TextLayout {
                justify: Justify::Right,
                ..default()
            },
            observe(|_trigger: On<Pointer<Click>>| {
                info!("Scoreboard clicked");
            }),
        ))
        .id();
    commands.entity(entry).set_parent_in_place(root);
    prev_entries.push(entry);
}
