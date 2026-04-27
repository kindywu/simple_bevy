mod client;
mod server;
mod shared;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::RepliconRenetPlugins;

use client::{
    check_connection, client_send_input, setup_scoreboard, start_client, update_scoreboard,
};
use server::{
    clamp_positions, combat_detection, respawn_dead_players, server_handle_input,
    server_on_connect, start_server,
};
use shared::{
    Dead, Direction, MoveInput, PlayerColor, PlayerCount, PlayerId, Position, Score,
    apply_position, setup_camera, spawn_render, update_visibility,
};

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Bevy 多人游戏".into(),
            ..default()
        }),
        ..default()
    }));

    app.add_plugins((RepliconPlugins, RepliconRenetPlugins));

    app.replicate::<Position>();
    app.replicate::<Direction>();
    app.replicate::<PlayerId>();
    app.replicate::<PlayerColor>();
    app.replicate::<Score>();
    app.replicate::<Dead>();

    app.add_client_message::<MoveInput>(Channel::Ordered);
    app.init_resource::<PlayerCount>();

    // 通用渲染
    app.add_systems(Startup, setup_camera);
    app.add_systems(Update, (spawn_render, apply_position));

    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("server") => {
            app.add_observer(server_on_connect);
            app.add_systems(Startup, (start_server, setup_scoreboard));
            app.add_systems(
                Update,
                (
                    server_handle_input,
                    clamp_positions,
                    combat_detection,
                    respawn_dead_players,
                    update_visibility,
                    update_scoreboard,
                )
                    .chain(),
            );
            info!("=== 服务端启动 ===");
        }
        Some("client") => {
            app.add_systems(Startup, (start_client, setup_scoreboard));
            app.add_systems(
                Update,
                (client_send_input, check_connection, update_visibility, update_scoreboard),
            );
            info!("=== 客户端启动 ===");
        }
        _ => {
            eprintln!("用法：cargo run -- server | client");
            std::process::exit(1);
        }
    }

    app.run();
}
