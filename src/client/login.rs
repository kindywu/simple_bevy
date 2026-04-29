use crate::shared::{AuthCredentials, PORT, PROTOCOL_ID};
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    RenetChannelsExt, RenetClient,
    netcode::{ClientAuthentication, NetcodeClientTransport},
    renet::ConnectionConfig,
};
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};

use super::{ConnectionState, ConnectTimer, LocalClientId};

#[derive(States, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameState {
    #[default]
    Login,
    InGame,
}

#[derive(Resource, Default)]
pub struct LoginData {
    pub username: String,
    pub password: String,
    pub focused: FocusField,
    pub status: Option<String>,
    pub connect_requested: bool,
}

#[derive(Default, PartialEq)]
pub enum FocusField {
    #[default]
    Username,
    Password,
}

#[derive(Component)]
pub(crate) struct LoginRoot;

#[derive(Component)]
pub(crate) struct LoginText;

fn char_for_key(key: KeyCode) -> Option<char> {
    match key {
        KeyCode::KeyA => Some('a'),
        KeyCode::KeyB => Some('b'),
        KeyCode::KeyC => Some('c'),
        KeyCode::KeyD => Some('d'),
        KeyCode::KeyE => Some('e'),
        KeyCode::KeyF => Some('f'),
        KeyCode::KeyG => Some('g'),
        KeyCode::KeyH => Some('h'),
        KeyCode::KeyI => Some('i'),
        KeyCode::KeyJ => Some('j'),
        KeyCode::KeyK => Some('k'),
        KeyCode::KeyL => Some('l'),
        KeyCode::KeyM => Some('m'),
        KeyCode::KeyN => Some('n'),
        KeyCode::KeyO => Some('o'),
        KeyCode::KeyP => Some('p'),
        KeyCode::KeyQ => Some('q'),
        KeyCode::KeyR => Some('r'),
        KeyCode::KeyS => Some('s'),
        KeyCode::KeyT => Some('t'),
        KeyCode::KeyU => Some('u'),
        KeyCode::KeyV => Some('v'),
        KeyCode::KeyW => Some('w'),
        KeyCode::KeyX => Some('x'),
        KeyCode::KeyY => Some('y'),
        KeyCode::KeyZ => Some('z'),
        KeyCode::Digit0 => Some('0'),
        KeyCode::Digit1 => Some('1'),
        KeyCode::Digit2 => Some('2'),
        KeyCode::Digit3 => Some('3'),
        KeyCode::Digit4 => Some('4'),
        KeyCode::Digit5 => Some('5'),
        KeyCode::Digit6 => Some('6'),
        KeyCode::Digit7 => Some('7'),
        KeyCode::Digit8 => Some('8'),
        KeyCode::Digit9 => Some('9'),
        KeyCode::Space => Some(' '),
        _ => None,
    }
}

pub fn setup_login_screen(mut commands: Commands) {
    commands
        .spawn((
            LoginRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgb(0.05, 0.05, 0.1)),
            GlobalZIndex(20),
        ))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(12.0),
                    padding: UiRect::all(Val::Px(40.0)),
                    ..default()
                })
                .with_children(|panel| {
                    panel.spawn((
                        LoginText,
                        Text::new("Login"),
                        TextFont::from_font_size(36.0),
                        TextColor(Color::WHITE),
                    ));

                    panel.spawn((
                        LoginText,
                        Text::new("Username:"),
                        TextFont::from_font_size(18.0),
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                    ));

                    panel.spawn((
                        LoginText,
                        Text::new(""),
                        TextFont::from_font_size(24.0),
                        TextColor(Color::WHITE),
                    ));

                    panel.spawn((
                        LoginText,
                        Text::new("Password:"),
                        TextFont::from_font_size(18.0),
                        TextColor(Color::srgb(0.7, 0.7, 0.7)),
                    ));

                    panel.spawn((
                        LoginText,
                        Text::new(""),
                        TextFont::from_font_size(24.0),
                        TextColor(Color::WHITE),
                    ));

                    panel.spawn((
                        LoginText,
                        Text::new(""),
                        TextFont::from_font_size(16.0),
                        TextColor(Color::srgb(1.0, 0.3, 0.3)),
                    ));

                    panel.spawn((
                        LoginText,
                        Text::new("[Tab] Switch field  [Enter] Connect"),
                        TextFont::from_font_size(14.0),
                        TextColor(Color::srgb(0.4, 0.4, 0.4)),
                    ));
                });
        });
}

pub fn handle_login_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut login_data: ResMut<LoginData>,
) {
    if login_data.connect_requested {
        return;
    }

    let just_pressed = |code: KeyCode| -> bool { keys.just_pressed(code) };

    if just_pressed(KeyCode::Tab) {
        login_data.focused = match login_data.focused {
            FocusField::Username => FocusField::Password,
            FocusField::Password => FocusField::Username,
        };
        login_data.status = None;
        return;
    }

    if just_pressed(KeyCode::Backspace) {
        login_data.status = None;
        match login_data.focused {
            FocusField::Username => {
                login_data.username.pop();
            }
            FocusField::Password => {
                login_data.password.pop();
            }
        }
        return;
    }

    if just_pressed(KeyCode::Enter) {
        if login_data.username.is_empty() || login_data.password.is_empty() {
            login_data.status = Some("Enter username and password".into());
        } else {
            login_data.connect_requested = true;
            login_data.status = Some("Connecting...".into());
        }
        return;
    }

    // Character input
    let char_keys = [
        KeyCode::KeyA, KeyCode::KeyB, KeyCode::KeyC, KeyCode::KeyD, KeyCode::KeyE,
        KeyCode::KeyF, KeyCode::KeyG, KeyCode::KeyH, KeyCode::KeyI, KeyCode::KeyJ,
        KeyCode::KeyK, KeyCode::KeyL, KeyCode::KeyM, KeyCode::KeyN, KeyCode::KeyO,
        KeyCode::KeyP, KeyCode::KeyQ, KeyCode::KeyR, KeyCode::KeyS, KeyCode::KeyT,
        KeyCode::KeyU, KeyCode::KeyV, KeyCode::KeyW, KeyCode::KeyX, KeyCode::KeyY,
        KeyCode::KeyZ,
        KeyCode::Digit0, KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4,
        KeyCode::Digit5, KeyCode::Digit6, KeyCode::Digit7, KeyCode::Digit8, KeyCode::Digit9,
        KeyCode::Space,
    ];
    for code in char_keys {
        if just_pressed(code) {
            if let Some(ch) = char_for_key(code) {
                login_data.status = None;
                match login_data.focused {
                    FocusField::Username => login_data.username.push(ch),
                    FocusField::Password => login_data.password.push(ch),
                }
            }
        }
    }
}

pub fn render_login_text(
    login_data: Res<LoginData>,
    mut texts: Query<&mut Text, With<LoginText>>,
) {
    let mut iter = texts.iter_mut();
    let _ = iter.next(); // title
    let _ = iter.next(); // "Username:"
    let mut username_text = iter.next().unwrap();
    let _ = iter.next(); // "Password:"
    let mut password_text = iter.next().unwrap();
    let mut status_text = iter.next().unwrap();
    let _ = iter.next(); // help text

    let username_display = if login_data.focused == FocusField::Username {
        format!("{}|", login_data.username)
    } else {
        login_data.username.clone()
    };

    let password_display = if login_data.focused == FocusField::Password {
        format!("{}|", "*".repeat(login_data.password.len()))
    } else {
        "*".repeat(login_data.password.len())
    };

    **username_text = username_display;
    **password_text = password_display;
    **status_text = login_data.status.clone().unwrap_or_default();
}

pub fn handle_connect(
    mut commands: Commands,
    login_data: ResMut<LoginData>,
    channels: Res<RepliconChannels>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !login_data.connect_requested {
        return;
    }

    let creds = AuthCredentials {
        username: login_data.username.clone(),
        password: login_data.password.clone(),
    };

    let creds_json = serde_json::to_string(&creds).unwrap();
    let creds_bytes = creds_json.as_bytes();
    let mut user_data = [0u8; 256];
    let len = creds_bytes.len().min(256);
    user_data[..len].copy_from_slice(&creds_bytes[..len]);

    let client = RenetClient::new(ConnectionConfig {
        server_channels_config: channels.server_configs(),
        client_channels_config: channels.client_configs(),
        ..default()
    });

    let server_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), PORT);
    let socket = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap();
    let client_id = now.subsec_nanos() as u64 + now.as_secs() * 1_000_000_000;

    let transport = NetcodeClientTransport::new(
        now,
        ClientAuthentication::Unsecure {
            client_id,
            protocol_id: PROTOCOL_ID,
            server_addr,
            user_data: Some(user_data),
        },
        socket,
    )
    .unwrap();

    commands.insert_resource(client);
    commands.insert_resource(transport);
    commands.insert_resource(ConnectionState::default());
    commands.insert_resource(ConnectTimer(Timer::from_seconds(5.0, TimerMode::Once)));
    commands.insert_resource(LocalClientId(client_id));

    next_state.set(GameState::InGame);
}

pub fn cleanup_login(mut commands: Commands, roots: Query<Entity, With<LoginRoot>>) {
    for entity in roots.iter() {
        commands.entity(entity).despawn();
    }
}
