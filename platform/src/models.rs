use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone, Serialize, Deserialize, Debug, FromRow)]
pub struct Player {
    pub username: String,
    pub password_hash: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PlayerDb {
    pub players: Vec<Player>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ApiKeyDb {
    pub keys: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct KeyVerifyResponse {
    pub valid: bool,
    pub message: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AuthResponse {
    pub success: bool,
    pub username: String,
    pub message: String,
    pub token: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RenewRequest {
    pub token: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RenewResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Clone)]
pub struct AppState {
    pub players: Arc<Mutex<PlayerDb>>,
    pub api_keys: ApiKeyDb,
    pub sessions: Arc<Mutex<HashMap<String, Session>>>, // token -> Session
}

#[derive(Clone, Debug)]
pub struct Session {
    pub username: String,
    pub expires_at: Instant,
}

pub const SESSION_TTL_SECS: u64 = 60;
