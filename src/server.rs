use crate::shared::*;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    RenetChannelsExt, RenetServer,
    netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig},
    renet::ConnectionConfig,
};
use rand::RngExt;
use std::{
    collections::HashSet,
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    time::SystemTime,
};

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

pub fn server_on_connect(
    trigger: On<Add, ConnectedClient>,
    mut commands: Commands,
    mut count: ResMut<PlayerCount>,
) {
    let client_entity = trigger.event_target();
    let id_num = client_entity.to_bits();

    let hue = (count.0 as f32 * 137.508) % 360.0;
    count.0 += 1;
    let (r, g, b) = hsv_to_rgb(hue, 0.8, 0.9);

    commands.spawn((
        Replicated,
        PlayerId(id_num),
        Position { x: 0.0, y: 0.0 },
        Direction::default(),
        PlayerColor { r, g, b },
        Score::default(),
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

// --- 战斗辅助函数 ---

type Point2 = (f32, f32);

fn tip_world(pos: &Position, dir: &Direction) -> Point2 {
    let (sin_a, cos_a) = dir.angle.sin_cos();
    (pos.x - 20.0 * sin_a, pos.y + 20.0 * cos_a)
}

fn triangle_vertices(pos: &Position, dir: &Direction) -> (Point2, Point2, Point2) {
    let (sin_a, cos_a) = dir.angle.sin_cos();
    let px = pos.x;
    let py = pos.y;
    // Local vertices: (0,20), (-15,-20), (15,-20) rotated by angle + translated
    let tip = (px - 20.0 * sin_a, py + 20.0 * cos_a);
    let bl = (
        px - 15.0 * cos_a + 20.0 * sin_a,
        py - 15.0 * sin_a - 20.0 * cos_a,
    );
    let br = (
        px + 15.0 * cos_a + 20.0 * sin_a,
        py + 15.0 * sin_a - 20.0 * cos_a,
    );
    (tip, bl, br)
}

fn point_in_triangle(p: Point2, a: Point2, b: Point2, c: Point2) -> bool {
    let (px, py) = p;
    let (ax, ay) = a;
    let (bx, by) = b;
    let (cx, cy) = c;

    // Vectors from A to B, A to C, A to P
    let v0x = cx - ax;
    let v0y = cy - ay;
    let v1x = bx - ax;
    let v1y = by - ay;
    let v2x = px - ax;
    let v2y = py - ay;

    // Dot products
    let dot00 = v0x * v0x + v0y * v0y;
    let dot01 = v0x * v1x + v0y * v1y;
    let dot02 = v0x * v2x + v0y * v2y;
    let dot11 = v1x * v1x + v1y * v1y;
    let dot12 = v1x * v2x + v1y * v2y;

    let denom = dot00 * dot11 - dot01 * dot01;
    if denom.abs() < f32::EPSILON {
        return false;
    }
    let inv_denom = 1.0 / denom;
    let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;

    u >= 0.0 && v >= 0.0 && (u + v) <= 1.0
}

fn find_safe_spawn(alive_positions: &[Position]) -> Position {
    let min_x = -VISIBLE_HALF_WIDTH + BOUNDARY_MARGIN;
    let max_x = VISIBLE_HALF_WIDTH - BOUNDARY_MARGIN;
    let min_y = -VISIBLE_HALF_HEIGHT + BOUNDARY_MARGIN;
    let max_y = VISIBLE_HALF_HEIGHT - BOUNDARY_MARGIN;
    let mut rng = rand::rng();

    for _ in 0..MAX_SPAWN_ATTEMPTS {
        let x = rng.random_range(min_x..max_x);
        let y = rng.random_range(min_y..max_y);

        let safe = alive_positions.iter().all(|p| {
            let dx = x - p.x;
            let dy = y - p.y;
            (dx * dx + dy * dy).sqrt() >= SAFE_SPAWN_DISTANCE
        });

        if safe {
            return Position { x, y };
        }
    }

    // 兜底：即使不太安全也返回一个随机位置
    Position {
        x: rng.random_range(min_x..max_x),
        y: rng.random_range(min_y..max_y),
    }
}

// --- 战斗系统（服务端） ---

pub fn combat_detection(
    mut commands: Commands,
    players: Query<(Entity, &Position, &Direction), Without<Dead>>,
    mut score_query: Query<&mut Score>,
) {
    let entries: Vec<_> = players.iter().collect();
    if entries.len() < 2 {
        return;
    }

    let mut killed: HashSet<Entity> = HashSet::new();
    let mut score_deltas: Vec<(Entity, u32)> = Vec::new();

    for i in 0..entries.len() {
        for j in (i + 1)..entries.len() {
            let (e_a, pos_a, dir_a) = entries[i];
            let (e_b, pos_b, dir_b) = entries[j];

            // 任一方已在本帧被杀，不再参与判定
            let a_killed = killed.contains(&e_a);
            let b_killed = killed.contains(&e_b);
            if a_killed && b_killed {
                continue;
            }

            let tip_a = tip_world(pos_a, dir_a);
            let (v0_b, v1_b, v2_b) = triangle_vertices(pos_b, dir_b);
            let tip_b = tip_world(pos_b, dir_b);
            let (v0_a, v1_a, v2_a) = triangle_vertices(pos_a, dir_a);

            let a_hits_b = point_in_triangle(tip_a, v0_b, v1_b, v2_b);
            let b_hits_a = point_in_triangle(tip_b, v0_a, v1_a, v2_a);

            match (a_hits_b, b_hits_a) {
                (true, true) => {
                    info!("💀 同归于尽: {:?} 和 {:?}", e_a, e_b);
                    killed.insert(e_a);
                    killed.insert(e_b);
                }
                (true, false) => {
                    if !a_killed && !b_killed {
                        info!("🔫 击杀: {:?} → {:?}", e_a, e_b);
                        score_deltas.push((e_a, KILL_SCORE));
                        killed.insert(e_b);
                    }
                }
                (false, true) => {
                    if !a_killed && !b_killed {
                        info!("🔫 击杀: {:?} → {:?}", e_b, e_a);
                        score_deltas.push((e_b, KILL_SCORE));
                        killed.insert(e_a);
                    }
                }
                (false, false) => {}
            }
        }
    }

    // 应用结果
    for entity in &killed {
        commands.entity(*entity).insert((
            Dead,
            RespawnTimer(Timer::from_seconds(RESPAWN_DELAY_SECS, TimerMode::Once)),
        ));
    }
    for (entity, delta) in &score_deltas {
        if let Ok(mut score) = score_query.get_mut(*entity) {
            score.0 += delta;
            info!("🏆 {:?} 得分: {} (总分: {})", entity, delta, score.0);
        }
    }
}

pub fn respawn_dead_players(
    time: Res<Time>,
    mut commands: Commands,
    mut dead_players: Query<(Entity, &mut RespawnTimer), With<Dead>>,
    alive_players: Query<&Position, Without<Dead>>,
) {
    for (entity, mut timer) in dead_players.iter_mut() {
        timer.0.tick(time.delta());
        if timer.0.just_finished() {
            let positions: Vec<Position> = alive_players.iter().copied().collect();
            let new_pos = find_safe_spawn(&positions);
            commands
                .entity(entity)
                .remove::<(Dead, RespawnTimer)>()
                .insert(new_pos);
            info!("♻️ {:?} 重生在 ({:.0}, {:.0})", entity, new_pos.x, new_pos.y);
        }
    }
}
