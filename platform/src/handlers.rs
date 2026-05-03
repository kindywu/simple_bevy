use crate::auth::{check_api_key, cleanup_expired_sessions, generate_token, hash_password};
use crate::models::{
    AppState, AuthResponse, KeyVerifyResponse, LoginRequest, RenewRequest, RenewResponse, Session,
    SESSION_TTL_SECS,
};
use axum::{Json, extract::State, http::StatusCode};
use std::time::{Duration, Instant};

// #[axum::debug_handler]
/// 验证服务端 API Key 是否有效
pub async fn verify_key_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
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
pub async fn login_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
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
pub async fn renew_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
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

pub async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}
