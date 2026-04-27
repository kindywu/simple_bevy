use crate::shared::*;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    RenetChannelsExt, RenetClient,
    netcode::{ClientAuthentication, NetcodeClientTransport},
    renet::ConnectionConfig,
};
use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    time::SystemTime,
};

pub fn start_client(world: &mut World) {
    let channels = world.resource::<RepliconChannels>();
    let client = RenetClient::new(ConnectionConfig {
        server_channels_config: channels.server_configs(),
        client_channels_config: channels.client_configs(),
        ..default()
    });

    let server_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), PORT);
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    let client_id = now.subsec_nanos() as u64 + now.as_secs() * 1_000_000_000;
    let transport = NetcodeClientTransport::new(
        now,
        ClientAuthentication::Unsecure {
            client_id,
            protocol_id: PROTOCOL_ID,
            server_addr,
            user_data: None,
        },
        socket,
    )
    .unwrap();

    world.insert_resource(client);
    world.insert_resource(transport);
    world.insert_resource(ConnectionState::default());
    world.insert_resource(ConnectTimer(Timer::from_seconds(5.0, TimerMode::Once)));
    world.insert_resource(LocalClientId(client_id));
}

pub fn client_send_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut writer: MessageWriter<MoveInput>,
    mut local_players: Query<&mut Direction, (With<LocalPlayer>, Without<Dead>)>,
) {
    let mut dx: f32 = 0.0;
    let mut dy: f32 = 0.0;

    if keyboard.pressed(KeyCode::KeyW) || keyboard.pressed(KeyCode::ArrowUp) {
        dy += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) || keyboard.pressed(KeyCode::ArrowDown) {
        dy -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
        dx -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
        dx += 1.0;
    }

    if dx != 0.0 || dy != 0.0 {
        let len: f32 = (dx * dx + dy * dy).sqrt();
        let ndx = dx / len;
        let ndy = dy / len;

        let angle = ndy.atan2(ndx) - std::f32::consts::FRAC_PI_2;
        for mut dir in local_players.iter_mut() {
            dir.angle = angle;
        }

        writer.write(MoveInput { dx: ndx, dy: ndy });
    }
}

#[derive(Component)]
pub(crate) struct ScoreboardRoot;

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

pub fn check_connection(
    time: Res<Time>,
    mut timer: ResMut<ConnectTimer>,
    client: Res<RenetClient>,
    mut state: ResMut<ConnectionState>,
) {
    timer.0.tick(time.delta());
    if client.is_connected() && !state.printed_connected {
        info!("✅ 已连接服务器");
        state.printed_connected = true;
    }
    if timer.0.is_finished() && !client.is_connected() {
        panic!("❌ 连接超时");
    }
}
