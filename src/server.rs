use crate::shared::*;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    RenetChannelsExt, RenetServer,
    netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig},
    renet::ConnectionConfig,
};
use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    time::SystemTime,
};

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
    ));

    info!("玩家连接 ID: {}", id_num);
}

pub fn server_handle_input(
    mut move_msgs: MessageReader<FromClient<MoveInput>>,
    mut players: Query<(&PlayerId, &mut Position, &mut Direction)>,
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

pub fn clamp_positions(mut players: Query<&mut Position, With<PlayerId>>) {
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
