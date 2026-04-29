use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Player {
    username: String,
    password_hash: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct PlayerDb {
    players: Vec<Player>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct ApiKeyDb {
    keys: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct AuthRequest {
    username: String,
    password: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct AuthResponse {
    success: bool,
    username: String,
    message: String,
}

#[derive(Clone)]
struct AppState {
    players: Arc<Mutex<PlayerDb>>,
    api_keys: ApiKeyDb,
}

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hex::encode(hasher.finalize())
}

fn load_players(path: &str) -> PlayerDb {
    let contents = std::fs::read_to_string(path).unwrap_or_default();
    let mut db: PlayerDb = serde_json::from_str(&contents).unwrap_or(PlayerDb { players: vec![] });
    let mut changed = false;
    for player in &mut db.players {
        if player.password_hash.is_empty() {
            player.password_hash = hash_password(&player.username);
            changed = true;
        }
    }
    if changed || db.players.is_empty() {
        if db.players.is_empty() {
            db.players = vec!["kindy", "ananda", "martin", "amy"]
                .into_iter()
                .map(|name| Player {
                    username: name.to_string(),
                    password_hash: hash_password(name),
                })
                .collect();
        }
        let json = serde_json::to_string_pretty(&db).unwrap();
        std::fs::write(path, json).unwrap();
    }
    db
}

fn load_api_keys(path: &str) -> ApiKeyDb {
    let contents = std::fs::read_to_string(path).unwrap_or_default();
    serde_json::from_str(&contents).unwrap_or(ApiKeyDb { keys: vec![] })
}

async fn auth_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(creds): Json<AuthRequest>,
) -> (StatusCode, Json<AuthResponse>) {
    let auth_header = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let token = match auth_header {
        Some(t) => t,
        None => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(AuthResponse {
                    success: false,
                    username: creds.username,
                    message: "Missing API key".into(),
                }),
            );
        }
    };

    if !state.api_keys.keys.iter().any(|k| k == token) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(AuthResponse {
                success: false,
                username: creds.username,
                message: "Invalid API key".into(),
            }),
        );
    }

    if creds.username.is_empty() || creds.password.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(AuthResponse {
                success: false,
                username: creds.username,
                message: "Username and password are required".into(),
            }),
        );
    }

    let password_hash = hash_password(&creds.password);
    let players = state.players.lock().unwrap();
    let valid = players
        .players
        .iter()
        .any(|p| p.username == creds.username && p.password_hash == password_hash);

    if valid {
        (
            StatusCode::OK,
            Json(AuthResponse {
                success: true,
                username: creds.username.clone(),
                message: "Authentication successful".into(),
            }),
        )
    } else {
        (
            StatusCode::FORBIDDEN,
            Json(AuthResponse {
                success: false,
                username: creds.username,
                message: "Invalid username or password".into(),
            }),
        )
    }
}

async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[tokio::main]
async fn main() {
    let players_path = format!("{MANIFEST_DIR}/players.json");
    let api_keys_path = format!("{MANIFEST_DIR}/api_keys.json");

    let players = load_players(&players_path);
    let api_keys = load_api_keys(&api_keys_path);

    println!("Platform started");
    println!("  {} players loaded:", players.players.len());
    for p in &players.players {
        println!("    - {}", p.username);
    }
    println!("  {} API keys loaded", api_keys.keys.len());

    let state = AppState {
        players: Arc::new(Mutex::new(players)),
        api_keys,
    };

    let app = Router::new()
        .route("/api/auth", post(auth_handler))
        .route("/api/health", get(health_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001")
        .await
        .expect("Failed to bind platform port");
    println!("Platform listening on http://127.0.0.1:3001");

    axum::serve(listener, app).await.unwrap();
}
