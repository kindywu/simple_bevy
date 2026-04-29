use crate::shared::*;
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::RenetClient;

mod login;
mod render;
mod scoreboard;

use login::{GameState, LoginData, cleanup_login, handle_connect, handle_login_input, render_login_text, setup_login_screen};
use render::{spawn_render, apply_position, update_visibility};
use scoreboard::{setup_scoreboard, update_scoreboard};

#[derive(Resource)]
pub(crate) struct ConnectTimer(pub Timer);

#[derive(Resource, Default)]
pub(crate) struct ConnectionState {
    pub printed_connected: bool,
}

#[derive(Resource)]
pub struct LocalClientId(pub u64);

pub fn client_send_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut writer: MessageWriter<MoveInput>,
    mut local_players: Query<&mut Direction, (With<render::LocalPlayer>, Without<Dead>)>,
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

pub fn check_connection(
    time: Res<Time>,
    mut timer: ResMut<ConnectTimer>,
    client: Res<RenetClient>,
    mut state: ResMut<ConnectionState>,
    mut next_state: ResMut<NextState<GameState>>,
    mut login_data: ResMut<LoginData>,
    mut commands: Commands,
) {
    timer.0.tick(time.delta());
    if client.is_connected() && !state.printed_connected {
        info!("已连接服务器");
        state.printed_connected = true;
    }
    let should_disconnect = (timer.0.is_finished() && !client.is_connected())
        || (state.printed_connected && !client.is_connected());
    if should_disconnect {
        if state.printed_connected {
            error!("与服务器断开(认证失败或服务器关闭)");
            login_data.status = Some("Disconnected (auth failed or server closed)".into());
        } else {
            error!("连接超时");
            login_data.status = Some("Connection timed out".into());
        }
        login_data.connect_requested = false;
        commands.remove_resource::<RenetClient>();
        commands.remove_resource::<bevy_replicon_renet::netcode::NetcodeClientTransport>();
        commands.remove_resource::<ConnectionState>();
        commands.remove_resource::<ConnectTimer>();
        commands.remove_resource::<LocalClientId>();
        next_state.set(GameState::Login);
    }
}

pub fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, Transform::default(), GlobalTransform::default()));
}

pub fn run() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Bevy 多人游戏 - 客户端".into(),
            ..default()
        }),
        ..default()
    }));

    app.add_plugins((RepliconPlugins, bevy_replicon_renet::RepliconRenetPlugins));

    app.replicate::<Position>();
    app.replicate::<Direction>();
    app.replicate::<PlayerId>();
    app.replicate::<PlayerColor>();
    app.replicate::<Score>();
    app.replicate::<Dead>();
    app.replicate::<PlayerName>();

    app.add_client_message::<MoveInput>(Channel::Ordered);
    app.init_resource::<PlayerCount>();

    app.init_state::<GameState>();
    app.init_resource::<LoginData>();

    app.add_systems(Startup, (setup_camera, setup_scoreboard));
    app.add_systems(OnEnter(GameState::Login), setup_login_screen);
    app.add_systems(OnExit(GameState::Login), cleanup_login);
    app.add_systems(
        Update,
        (handle_login_input, render_login_text, handle_connect).run_if(in_state(GameState::Login)),
    );
    app.add_systems(
        Update,
        (client_send_input, check_connection, spawn_render, apply_position, update_visibility, update_scoreboard)
            .run_if(in_state(GameState::InGame)),
    );

    info!("=== 客户端启动 ===");

    app.run();
}
