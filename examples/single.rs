use bevy::{app::ScheduleRunnerPlugin, prelude::*, state::app::StatesPlugin};
use bevy_replicon::prelude::*;
use bevy_replicon::shared::backend::connected_client::ConnectedClient;
use bevy_replicon::shared::message::client_message::ClientMessageAppExt;
use bevy_replicon_renet::{
    RenetChannelsExt, RenetClient, RenetServer, RepliconRenetPlugins,
    netcode::{
        ClientAuthentication, NetcodeClientTransport, NetcodeServerTransport, ServerAuthentication,
        ServerConfig,
    },
    renet::ConnectionConfig,
};
use serde::{Deserialize, Serialize};
use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    time::SystemTime,
};

const PORT: u16 = 5000;
const MOVE_SPEED: f32 = 300.0;
const PROTOCOL_ID: u64 = 123456;

#[derive(Component, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Resource, Default)]
struct PlayerCount(u32);

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

#[derive(Resource)]
struct ConnectTimer(Timer);

#[derive(Resource, Default)]
struct ConnectionState {
    printed_connected: bool,
}

fn main() {
    let mut app = App::new();
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(|s| s.as_str()) {
        Some("server") => {
            app.add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
                std::time::Duration::from_secs_f64(1.0 / 60.0),
            )));
            app.add_plugins(bevy::log::LogPlugin::default());
            app.add_plugins(StatesPlugin);
            app.init_resource::<PlayerCount>();
            app.add_observer(server_on_connect);
            app.add_systems(Startup, start_server);
            app.add_systems(Update, server_handle_input);
        }
        Some("client") => {
            app.add_plugins(DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy 多人游戏".into(),
                    ..default()
                }),
                ..default()
            }));
            app.add_systems(Startup, start_client);
            app.add_systems(Startup, setup_camera);
            app.add_systems(
                Update,
                (
                    client_send_input,
                    check_connection,
                    client_spawn_render,
                    client_apply_position,
                ),
            );
        }
        _ => {
            eprintln!("用法：cargo run -- server | client");
            std::process::exit(1);
        }
    }

    app.add_plugins((RepliconPlugins, RepliconRenetPlugins));
    app.replicate::<Position>()
        .replicate::<PlayerId>()
        .replicate::<PlayerColor>();
    app.add_client_message::<MoveInput>(Channel::Ordered);

    app.run();
}

fn start_server(world: &mut World) {
    let channels = world.resource::<RepliconChannels>();
    let server_configs = channels.server_configs();
    let client_configs = channels.client_configs();
    let server = RenetServer::new(ConnectionConfig {
        server_channels_config: server_configs,
        client_channels_config: client_configs,
        ..default()
    });
    let bind_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), PORT);
    let socket = UdpSocket::bind(bind_addr).expect("端口绑定失败");
    info!("服务器监听于 {:?}", socket.local_addr());
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    let transport = NetcodeServerTransport::new(
        ServerConfig {
            current_time: now,
            max_clients: 8,
            protocol_id: PROTOCOL_ID,
            public_addresses: vec![bind_addr],
            authentication: ServerAuthentication::Unsecure,
        },
        socket,
    )
    .expect("传输层创建失败");
    world.insert_resource(server);
    world.insert_resource(transport);
}

fn server_on_connect(
    trigger: On<Add, ConnectedClient>,
    mut commands: Commands,
    mut count: ResMut<PlayerCount>,
) {
    let client_entity = trigger.event_target();
    info!("🔗 客户端连接! Client entity: {:?}", client_entity);
    let id_num = client_entity.to_bits();

    let hue = (count.0 as f32 * 137.508) % 360.0;
    count.0 += 1;

    let (r, g, b) = hsv_to_rgb(hue, 0.8, 0.9);
    commands.spawn((
        Replicated,
        PlayerId(id_num),
        Position { x: 0.0, y: 0.0 },
        PlayerColor { r, g, b },
    ));
    info!("🎮 服务器 spawn PlayerId: {}", id_num);
}

fn server_handle_input(
    mut move_msgs: MessageReader<FromClient<MoveInput>>,
    mut players: Query<(&PlayerId, &mut Position)>,
    time: Res<Time>,
) {
    for FromClient { client_id, message } in move_msgs.read() {
        let sender_id = client_id_to_u64(*client_id);
        for (player_id, mut pos) in players.iter_mut() {
            if player_id.0 == sender_id {
                pos.x += message.dx * MOVE_SPEED * time.delta_secs();
                pos.y += message.dy * MOVE_SPEED * time.delta_secs();
            }
        }
    }
}

fn start_client(world: &mut World) {
    let channels = world.resource::<RepliconChannels>();
    let server_configs = channels.server_configs();
    let client_configs = channels.client_configs();
    let client = RenetClient::new(ConnectionConfig {
        server_channels_config: server_configs,
        client_channels_config: client_configs,
        ..default()
    });
    let server_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), PORT);
    let socket = UdpSocket::bind(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0))
        .expect("客户端 socket 绑定失败");
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
    .expect("客户端传输层创建失败");
    world.insert_resource(client);
    world.insert_resource(transport);
    world.insert_resource(ConnectionState::default());
    world.insert_resource(ConnectTimer(Timer::from_seconds(5.0, TimerMode::Once)));
    info!("客户端连接到 {}:{}", Ipv4Addr::LOCALHOST, PORT);
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Transform::default(), GlobalTransform::default()));
}

fn client_send_input(keyboard: Res<ButtonInput<KeyCode>>, mut writer: MessageWriter<MoveInput>) {
    let mut dx = 0.0_f32;
    let mut dy = 0.0_f32;
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
        let len = (dx * dx + dy * dy).sqrt();
        writer.write(MoveInput {
            dx: dx / len,
            dy: dy / len,
        });
    }
}

fn client_spawn_render(
    mut commands: Commands,
    new_players: Query<(Entity, &PlayerColor), (With<PlayerId>, Without<LocalSprite>)>,
) {
    for (entity, color) in new_players.iter() {
        info!("✅ 生成渲染实体: {:?}", entity);
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

fn client_apply_position(mut players: Query<(&Position, &mut Transform), With<PlayerId>>) {
    for (pos, mut transform) in players.iter_mut() {
        transform.translation = Vec3::new(pos.x, pos.y, 0.0);
    }
}

fn client_id_to_u64(id: ClientId) -> u64 {
    match id {
        ClientId::Server => 0,
        ClientId::Client(entity) => entity.to_bits(),
        #[allow(unreachable_patterns)]
        _ => {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            format!("{:?}", id).hash(&mut hasher);
            hasher.finish()
        }
    }
}

fn check_connection(
    time: Res<Time>,
    mut timer: ResMut<ConnectTimer>,
    client: Res<RenetClient>,
    mut state: ResMut<ConnectionState>,
) {
    timer.0.tick(time.delta());
    if client.is_connected() && !state.printed_connected {
        info!("✅ 已连接到服务器");
        state.printed_connected = true;
    }
    if timer.0.is_finished() && !client.is_connected() {
        panic!("❌ 连接服务器失败（超时 5 秒）");
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
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
