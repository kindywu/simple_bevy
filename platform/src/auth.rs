use crate::models::{AppState, Session};
use axum::http::{HeaderMap, StatusCode};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::time::Instant;

pub fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn generate_token() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut token = String::with_capacity(32);
    for _ in 0..32 {
        let idx = (rand::random::<u32>() as usize) % CHARSET.len();
        token.push(CHARSET[idx] as char);
    }
    token
}

pub fn cleanup_expired_sessions(sessions: &mut HashMap<String, Session>) {
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

pub fn extract_api_key(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

pub fn check_api_key(state: &AppState, headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    let token =
        extract_api_key(headers).ok_or((StatusCode::UNAUTHORIZED, "Missing API key".into()))?;
    if !state.api_keys.keys.iter().any(|k| k == token) {
        return Err((StatusCode::UNAUTHORIZED, "Invalid API key".into()));
    }
    Ok(())
}
