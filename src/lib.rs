mod shared;
#[cfg(feature = "server")]
mod server;
#[cfg(feature = "client")]
mod client;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::RepliconRenetPlugins;

use shared::{
    Dead, Direction, MoveInput, PlayerColor, PlayerCount, PlayerId, Position, Score,
    apply_position, setup_camera, setup_scoreboard, spawn_render, update_scoreboard,
    update_visibility,
};

fn common_setup(app: &mut App) {
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

    app.add_systems(Startup, setup_camera);
    app.add_systems(Update, (spawn_render, apply_position));
}

#[cfg(feature = "server")]
pub fn run_server() {
    use server::{
        clamp_positions, combat_detection, respawn_dead_players, server_handle_input,
        server_on_connect, start_server,
    };

    let mut app = App::new();
    common_setup(&mut app);

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

    app.run();
}

#[cfg(feature = "client")]
pub fn run_client() {
    use client::{check_connection, client_send_input, start_client};

    let mut app = App::new();
    common_setup(&mut app);

    app.add_systems(Startup, (start_client, setup_scoreboard));
    app.add_systems(
        Update,
        (client_send_input, check_connection, update_visibility, update_scoreboard),
    );
    info!("=== 客户端启动 ===");

    app.run();
}
