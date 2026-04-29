use crate::shared::{AuthCredentials, AuthResponse, PLATFORM_PORT};
use bevy::prelude::*;
use serde::Deserialize;

#[derive(Resource)]
pub struct ApiKey(pub String);

/// 标记平台是否可用。启动时验证失败则置为 false，后续玩家认证直接拒绝。
#[derive(Resource)]
pub struct PlatformConnected(pub bool);

#[derive(Deserialize)]
struct KeyVerifyResponse {
    valid: bool,
    message: String,
}

fn platform_url(path: &str) -> String {
    format!("http://127.0.0.1:{}{}", PLATFORM_PORT, path)
}

fn auth_header(api_key: &str) -> String {
    format!("Bearer {}", api_key)
}

/// 带重试的平台 Key 验证（启动时调用）
pub fn verify_api_key_with_retry(api_key: &str, max_retries: u32) -> Result<(), String> {
    for attempt in 1..=max_retries {
        match try_verify_key(api_key) {
            Ok(()) => return Ok(()),
            Err(msg) => {
                if attempt < max_retries {
                    warn!("平台连接失败 (尝试 {attempt}/{max_retries}): {msg}，1 秒后重试...");
                    std::thread::sleep(std::time::Duration::from_secs(1));
                } else {
                    error!("平台连接失败 (已重试 {max_retries} 次): {msg}");
                    return Err(msg);
                }
            }
        }
    }
    unreachable!()
}

fn try_verify_key(api_key: &str) -> Result<(), String> {
    let response = ureq::post(&platform_url("/api/auth/verify-key"))
        .set("Authorization", &auth_header(api_key))
        .call()
        .map_err(|e| format!("Platform unreachable: {}", e))?;

    let result: KeyVerifyResponse = response
        .into_json()
        .map_err(|e| format!("Invalid platform response: {}", e))?;

    if result.valid {
        Ok(())
    } else {
        Err(result.message)
    }
}

/// 验证玩家登录凭据
pub fn validate_credentials(api_key: &str, creds: &AuthCredentials) -> Result<String, String> {
    let body = serde_json::to_string(creds).map_err(|_| "Failed to serialize credentials".to_string())?;

    let response = ureq::post(&platform_url("/api/auth/login"))
        .set("Authorization", &auth_header(api_key))
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| format!("认证服务不可用: {}", e))?;

    let auth_response: AuthResponse = response
        .into_json()
        .map_err(|e| format!("Invalid platform response: {}", e))?;

    if auth_response.success {
        Ok(auth_response.username)
    } else {
        Err(auth_response.message)
    }
}
