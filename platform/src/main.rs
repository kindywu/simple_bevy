use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::{get, post},
};
use axum_server::tls_rustls::RustlsConfig;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct KeyVerifyResponse {
    valid: bool,
    message: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct AuthResponse {
    success: bool,
    username: String,
    message: String,
    token: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct RenewRequest {
    token: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct RenewResponse {
    success: bool,
    message: String,
}

#[derive(Clone)]
struct AppState {
    players: Arc<Mutex<PlayerDb>>,
    api_keys: ApiKeyDb,
    sessions: Arc<Mutex<HashMap<String, Session>>>, // token -> Session
}

#[derive(Clone, Debug)]
struct Session {
    username: String,
    expires_at: Instant,
}

const SESSION_TTL_SECS: u64 = 60;

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

fn extract_api_key(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

fn check_api_key(state: &AppState, headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    let token = extract_api_key(headers).ok_or((
        StatusCode::UNAUTHORIZED,
        "Missing API key".into(),
    ))?;
    if !state.api_keys.keys.iter().any(|k| k == token) {
        return Err((StatusCode::UNAUTHORIZED, "Invalid API key".into()));
    }
    Ok(())
}

fn generate_token() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut token = String::with_capacity(32);
    for _ in 0..32 {
        let idx = (rand::random::<u32>() as usize) % CHARSET.len();
        token.push(CHARSET[idx] as char);
    }
    token
}

fn cleanup_expired_sessions(sessions: &mut HashMap<String, Session>) {
    let now = Instant::now();
    let expired: Vec<String> = sessions
        .iter()
        .filter(|(_, s)| s.expires_at <= now)
        .map(|(k, _)| k.clone())
        .collect();
    for k in expired {
        sessions.remove(&k);
    }
}

/// 验证服务端 API Key 是否有效
async fn verify_key_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> (StatusCode, Json<KeyVerifyResponse>) {
    match check_api_key(&state, &headers) {
        Ok(()) => (
            StatusCode::OK,
            Json(KeyVerifyResponse {
                valid: true,
                message: "API key is valid".into(),
            }),
        ),
        Err((code, msg)) => (
            code,
            Json(KeyVerifyResponse {
                valid: false,
                message: msg,
            }),
        ),
    }
}

/// 验证玩家登录凭据
async fn login_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(creds): Json<LoginRequest>,
) -> (StatusCode, Json<AuthResponse>) {
    if let Err((code, msg)) = check_api_key(&state, &headers) {
        return (
            code,
            Json(AuthResponse {
                success: false,
                username: creds.username,
                message: msg,
                token: String::new(),
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
                token: String::new(),
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
        let mut sessions = state.sessions.lock().unwrap();
        cleanup_expired_sessions(&mut sessions);

        // 顶号：删除该用户已有的 session
        let old_tokens: Vec<String> = sessions
            .iter()
            .filter(|(_, s)| s.username == creds.username)
            .map(|(k, _)| k.clone())
            .collect();
        for old_token in old_tokens {
            sessions.remove(&old_token);
        }

        let token = generate_token();
        sessions.insert(
            token.clone(),
            Session {
                username: creds.username.clone(),
                expires_at: Instant::now() + Duration::from_secs(SESSION_TTL_SECS),
            },
        );

        (
            StatusCode::OK,
            Json(AuthResponse {
                success: true,
                username: creds.username.clone(),
                message: "Authentication successful".into(),
                token,
            }),
        )
    } else {
        (
            StatusCode::FORBIDDEN,
            Json(AuthResponse {
                success: false,
                username: creds.username,
                message: "Invalid username or password".into(),
                token: String::new(),
            }),
        )
    }
}

/// 续约 session
async fn renew_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RenewRequest>,
) -> (StatusCode, Json<RenewResponse>) {
    if let Err((code, msg)) = check_api_key(&state, &headers) {
        return (
            code,
            Json(RenewResponse {
                success: false,
                message: msg,
            }),
        );
    }

    let mut sessions = state.sessions.lock().unwrap();
    cleanup_expired_sessions(&mut sessions);

    if let Some(session) = sessions.get_mut(&req.token) {
        session.expires_at = Instant::now() + Duration::from_secs(SESSION_TTL_SECS);
        (
            StatusCode::OK,
            Json(RenewResponse {
                success: true,
                message: "Session renewed".into(),
            }),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(RenewResponse {
                success: false,
                message: "Session not found or expired".into(),
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
        sessions: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/api/auth/verify-key", post(verify_key_handler))
        .route("/api/auth/login", post(login_handler))
        .route("/api/session/renew", post(renew_handler))
        .route("/api/health", get(health_handler))
        .with_state(state);

    let tls_config = RustlsConfig::from_pem_file(
        format!("{MANIFEST_DIR}/certs/localhost.pem"),
        format!("{MANIFEST_DIR}/certs/localhost-key.pem"),
    )
    .await
    .expect("Failed to load TLS certificates (run: mkcert -install && cd platform/certs && mkcert localhost 127.0.0.1 ::1)");

    let addr = "127.0.0.1:3001".parse().unwrap();
    println!("Platform listening on https://127.0.0.1:3001");

    axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
