use bevy::prelude::*;
use bevy_ui_widgets::{Button, observe};
use shared::{PlayerColor, PlayerId, PlayerName, Score};

/// 排行榜根节点 Widget 标记
#[derive(Component)]
pub struct ScoreboardWidget;

/// 排行榜条目 Widget 标记
#[derive(Component)]
struct ScoreboardEntry;

/// 排行榜标题 Widget 标记
#[derive(Component)]
struct ScoreboardTitle;

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
        ScoreboardWidget,
    ));
}

fn spawn_title(commands: &mut Commands, parent: Entity, font: Handle<Font>) -> Entity {
    let title = commands
        .spawn((
            ScoreboardTitle,
            Text::new("=== 排行榜 ==="),
            TextFont {
                font,
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
    commands.entity(title).set_parent_in_place(parent);
    title
}

fn spawn_empty_hint(commands: &mut Commands, parent: Entity, font: Handle<Font>) -> Entity {
    let entry = commands
        .spawn((
            ScoreboardEntry,
            Button,
            Node {
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            Text::new("等待玩家加入..."),
            TextFont {
                font,
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
    commands.entity(entry).set_parent_in_place(parent);
    entry
}

fn spawn_player_entry(
    commands: &mut Commands,
    parent: Entity,
    font: Handle<Font>,
    rank: usize,
    name: &str,
    score: u32,
    color: &PlayerColor,
) -> Entity {
    let rank_str = match rank {
        0 => "\u{1f947}".to_string(),
        1 => "\u{1f948}".to_string(),
        2 => "\u{1f949}".to_string(),
        n => format!("#{}", n + 1),
    };
    let text_str = format!("{rank_str}  {name}  —  {score}分");
    let name_owned = name.to_string();

    let entry = commands
        .spawn((
            ScoreboardEntry,
            Button,
            Node {
                padding: UiRect::all(Val::Px(4.0)),
                ..default()
            },
            Text::new(text_str),
            TextFont {
                font,
                font_size: 17.0,
                ..default()
            },
            TextColor(Color::srgb(color.r, color.g, color.b)),
            TextLayout {
                justify: Justify::Center,
                ..default()
            },
            observe(move |_trigger: On<Pointer<Click>>| {
                info!("点击了排行榜条目: {name_owned}");
            }),
        ))
        .id();
    commands.entity(entry).set_parent_in_place(parent);
    entry
}

pub fn update_scoreboard(
    mut commands: Commands,
    scoreboard: Query<Entity, With<ScoreboardWidget>>,
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

    let title = spawn_title(&mut commands, root, font.clone());
    prev_entries.push(title);

    let mut player_data: Vec<_> = players.iter().collect();
    player_data.sort_unstable_by(|a, b| b.1.0.cmp(&a.1.0));

    if player_data.is_empty() {
        let entry = spawn_empty_hint(&mut commands, root, font.clone());
        prev_entries.push(entry);
    } else {
        for (rank, (_player_id, score, color, name)) in player_data.iter().enumerate() {
            let entry = spawn_player_entry(
                &mut commands,
                root,
                font.clone(),
                rank,
                &name.0,
                score.0,
                color,
            );
            prev_entries.push(entry);
        }
    }
}
