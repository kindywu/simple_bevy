use bevy::prelude::*;
use shared::{PlayerColor, PlayerId, PlayerName, Score};

#[derive(Component)]
pub struct ScoreboardRoot;

pub fn setup_scoreboard(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(4.0),
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
    mut font_handle: Local<Option<Handle<Font>>>,
) {
    let Ok(root) = scoreboard.single() else {
        return;
    };

    for &entity in prev_entries.iter() {
        commands.entity(entity).despawn();
    }
    prev_entries.clear();

    let font = font_handle
        .get_or_insert_with(|| asset_server.load("fonts/msyh.ttc"))
        .clone();

    let mut player_data: Vec<_> = players.iter().collect();
    player_data.sort_unstable_by(|a, b| b.1.0.cmp(&a.1.0));

    let title = commands
        .spawn((
            Text::new("=== 排行榜 ==="),
            TextFont {
                font: font.clone(),
                font_size: 22.0,
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.85, 0.3)),
            TextLayout {
                justify: Justify::Center,
                ..default()
            },
        ))
        .id();
    commands.entity(title).set_parent_in_place(root);
    prev_entries.push(title);

    if player_data.is_empty() {
        let entry = commands
            .spawn((
                Text::new("等待玩家加入..."),
                TextFont {
                    font: font.clone(),
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
                TextLayout {
                    justify: Justify::Center,
                    ..default()
                },
            ))
            .id();
        commands.entity(entry).set_parent_in_place(root);
        prev_entries.push(entry);
    } else {
        for (rank, (_player_id, score, color, name)) in player_data.iter().enumerate() {
            let rank_str = match rank {
                0 => "\u{1f947}".to_string(),
                1 => "\u{1f948}".to_string(),
                2 => "\u{1f949}".to_string(),
                n => format!("#{}", n + 1),
            };
            let text_str = format!("{rank_str}  {}  —  {}分", name.0, score.0);
            let entry = commands
                .spawn((
                    Text::new(text_str),
                    TextFont {
                        font: font.clone(),
                        font_size: 17.0,
                        ..default()
                    },
                    TextColor(Color::srgb(color.r, color.g, color.b)),
                    TextLayout {
                        justify: Justify::Center,
                        ..default()
                    },
                ))
                .id();
            commands.entity(entry).set_parent_in_place(root);
            prev_entries.push(entry);
        }
    }
}
