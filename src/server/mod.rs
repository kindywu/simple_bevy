use crate::shared::*;
use bevy::asset::{AssetPlugin, UnapprovedPathMode};
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon::shared::backend::connected_client::NetworkId;
use bevy_replicon_renet::{
    RenetChannelsExt, RenetServer,
    netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig},
    renet::ConnectionConfig,
};
use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    time::SystemTime,
};

mod auth;
mod bullet;
pub(crate) mod combat;
mod render;
mod scoreboard;

use auth::{ApiKey, PlatformConnected, verify_api_key_with_retry};
use bullet::{
    ShootCooldown, bullet_lifetime, bullet_player_collision, move_bullets, server_handle_shoot,
    tick_cooldowns,
};
use combat::{combat_detection, respawn_dead_players};
use render::{apply_position, setup_camera, spawn_bullet_render, spawn_render};
use scoreboard::{setup_scoreboard, update_scoreboard};

pub const MOVE_SPEED: f32 = 300.0;
pub const VISIBLE_HALF_WIDTH: f32 = 640.0;
pub const VISIBLE_HALF_HEIGHT: f32 = 360.0;
pub const BOUNDARY_MARGIN: f32 = 25.0;
pub const KILL_SCORE: u32 = 10;
pub const RESPAWN_DELAY_SECS: f32 = 3.0;
pub const SAFE_SPAWN_DISTANCE: f32 = 200.0;
pub const MAX_SPAWN_ATTEMPTS: u32 = 50;

#[derive(Component, Deref, DerefMut)]
pub struct RespawnTimer(pub Timer);

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

pub fn start_server(world: &mut World) {
    let channels = world.resource::<RepliconChannels>();
    let server = RenetServer::new(ConnectionConfig {
        server_channels_config: channels.server_configs(),
        client_channels_config: channels.client_configs(),
        ..default()
    });

    let bind_addr = SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), PORT);
    let socket = UdpSocket::bind(bind_addr).expect("端口绑定失败");
    info!("服务器监听: {:?}", socket.local_addr());

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
    .unwrap();

    world.insert_resource(server);
    world.insert_resource(transport);
}

fn verify_platform_connection(api_key: Res<ApiKey>, mut connected: ResMut<PlatformConnected>) {
    match verify_api_key_with_retry(&api_key.0, 3) {
        Ok(()) => {
            info!("平台连接验证成功");
            connected.0 = true;
        }
        Err(msg) => {
            error!("平台不可用: {msg} — 服务器继续运行，但玩家认证将不可用");
            connected.0 = false;
        }
    }
}

pub fn server_on_connect(
    trigger: On<Add, ConnectedClient>,
    mut commands: Commands,
    mut count: ResMut<PlayerCount>,
    transport: Res<NetcodeServerTransport>,
    api_key: Res<ApiKey>,
    platform: Res<PlatformConnected>,
    clients: Query<&NetworkId>,
    mut server: ResMut<RenetServer>,
) {
    let client_entity = trigger.event_target();
    let id_num = client_entity.to_bits();

    let validated_name = if let Ok(network_id) = clients.get(client_entity) {
        let client_id = network_id.get();
        if let Some(user_data) = transport.user_data(client_id) {
            let len = user_data.iter().position(|&b| b == 0).unwrap_or(256);
            match serde_json::from_slice::<AuthCredentials>(&user_data[..len]) {
                Ok(creds) => {
                    if !platform.0 {
                        error!("平台不可用，拒绝玩家 {} 的认证请求", creds.username);
                        server.disconnect(client_id);
                        return;
                    }
                    match auth::validate_credentials(&api_key.0, &creds) {
                        Ok(username) => {
                            info!("玩家认证成功: {}", username);
                            Some(username)
                        }
                        Err(msg) => {
                            error!("认证失败: {} - {}", creds.username, msg);
                            server.disconnect(client_id);
                            return;
                        }
                    }
                }
                Err(_) => {
                    info!("玩家连接(无认证信息) ID: {}", id_num);
                    None
                }
            }
        } else {
            info!("玩家连接(无认证信息) ID: {}", id_num);
            None
        }
    } else {
        info!("玩家连接(无认证信息) ID: {}", id_num);
        None
    };

    let name = validated_name.unwrap_or_else(|| format!("Player_{}", id_num));

    let hue = (count.0 as f32 * 137.508) % 360.0;
    count.0 += 1;
    let (r, g, b) = hsv_to_rgb(hue, 0.8, 0.9);

    commands.spawn((
        Replicated,
        PlayerId(id_num),
        PlayerName(name),
        Position { x: 0.0, y: 0.0 },
        Direction::default(),
        PlayerColor { r, g, b },
        Score::default(),
        Health(MAX_HP),
        ShootCooldown({
            let mut t = Timer::from_seconds(SHOOT_COOLDOWN_SECS, TimerMode::Once);
            t.tick(std::time::Duration::from_secs_f32(SHOOT_COOLDOWN_SECS));
            t
        }),
    ));

    info!("玩家连接 ID: {}", id_num);
}

pub fn server_handle_input(
    mut move_msgs: MessageReader<FromClient<MoveInput>>,
    mut players: Query<(&PlayerId, &mut Position, &mut Direction), Without<Dead>>,
    time: Res<Time>,
) {
    for FromClient { client_id, message } in move_msgs.read() {
        let sender_id = client_id_to_u64(*client_id);
        for (player_id, mut pos, mut dir) in players.iter_mut() {
            if player_id.0 == sender_id {
                pos.x += message.dx * MOVE_SPEED * time.delta_secs();
                pos.y += message.dy * MOVE_SPEED * time.delta_secs();
                if message.dx != 0.0 || message.dy != 0.0 {
                    dir.angle = message.dy.atan2(message.dx) - std::f32::consts::FRAC_PI_2;
                }
            }
        }
    }
}

pub fn clamp_positions(mut players: Query<&mut Position, (With<PlayerId>, Without<Dead>)>) {
    let min_x = -VISIBLE_HALF_WIDTH + BOUNDARY_MARGIN;
    let max_x = VISIBLE_HALF_WIDTH - BOUNDARY_MARGIN;
    let min_y = -VISIBLE_HALF_HEIGHT + BOUNDARY_MARGIN;
    let max_y = VISIBLE_HALF_HEIGHT - BOUNDARY_MARGIN;
    for mut pos in players.iter_mut() {
        pos.x = pos.x.clamp(min_x, max_x);
        pos.y = pos.y.clamp(min_y, max_y);
    }
}

fn client_id_to_u64(id: ClientId) -> u64 {
    match id {
        ClientId::Server => 0,
        ClientId::Client(entity) => entity.to_bits(),
    }
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

pub fn run(api_key: &str) {
    let mut app = App::new();

    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy 多人游戏 - 服务端".into(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                unapproved_path_mode: UnapprovedPathMode::Allow,
                ..default()
            }),
    );

    app.add_plugins((RepliconPlugins, bevy_replicon_renet::RepliconRenetPlugins));

    app.replicate::<Position>();
    app.replicate::<Direction>();
    app.replicate::<PlayerId>();
    app.replicate::<PlayerColor>();
    app.replicate::<Score>();
    app.replicate::<Dead>();
    app.replicate::<PlayerName>();
    app.replicate::<Health>();
    app.replicate::<Bullet>();
    app.replicate::<BulletOwner>();

    app.add_client_message::<MoveInput>(Channel::Ordered);
    app.add_client_message::<ShootInput>(Channel::Ordered);
    app.init_resource::<PlayerCount>();
    app.insert_resource(ApiKey(api_key.to_string()));
    app.insert_resource(PlatformConnected(true));

    app.add_observer(server_on_connect);
    app.add_systems(Startup, (setup_camera, start_server, setup_scoreboard, verify_platform_connection));
    app.add_systems(
        Update,
        (
            spawn_render,
            tick_cooldowns,
            server_handle_input,
            server_handle_shoot,
            spawn_bullet_render,
            move_bullets,
            clamp_positions,
            bullet_player_collision,
            combat_detection,
            bullet_lifetime,
            respawn_dead_players,
            apply_position,
            update_visibility,
            update_scoreboard,
        )
            .chain(),
    );
    info!("=== 服务端启动 ===");

    app.run();
}
