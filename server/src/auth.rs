use shared::{AuthCredentials, AuthResponse, PLATFORM_HOST, PLATFORM_PORT, RenewRequest, RenewResponse};
use bevy::prelude::*;
use rustls::{ClientConfig, RootCertStore};
use serde::Deserialize;
use std::sync::{Arc, OnceLock};

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
    format!("https://{}:{}{}", PLATFORM_HOST, PLATFORM_PORT, path)
}

fn auth_header(api_key: &str) -> String {
    format!("Bearer {}", api_key)
}

/// 构建信任 mkcert CA 的 ureq Agent（延迟初始化，全局复用）
fn tls_agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        let mut root_store = RootCertStore::empty();

        // 加载系统原生根证书（生产环境的 Let's Encrypt 等）
        let native = rustls_native_certs::load_native_certs();
        for cert in native.certs {
            let _ = root_store.add(cert);
        }

        // 加载 mkcert 本地 CA（开发环境）
        if let Some(path) = mkcert_ca_path() {
            if let Ok(pem) = std::fs::read(&path) {
                for cert in rustls_pemfile::certs(&mut pem.as_slice()) {
                    if let Ok(cert) = cert {
                        let _ = root_store.add(cert);
                    }
                }
            }
        }

        let _ = rustls::crypto::ring::default_provider().install_default();

        let tls_config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        ureq::AgentBuilder::new()
            .tls_config(Arc::new(tls_config))
            .build()
    })
}

/// 查找 mkcert 根 CA 证书路径
fn mkcert_ca_path() -> Option<std::path::PathBuf> {
    if let Ok(p) = std::env::var("MKCERT_CA") {
        return Some(p.into());
    }
    if let Ok(root) = std::env::var("LOCALAPPDATA") {
        let p = std::path::PathBuf::from(root).join("mkcert/rootCA.pem");
        if p.exists() {
            return Some(p);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        for sub in &[
            ".local/share/mkcert/rootCA.pem",
            "Library/Application Support/mkcert/rootCA.pem",
        ] {
            let p = std::path::PathBuf::from(&home).join(sub);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
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
    let response = tls_agent()
        .post(&platform_url("/api/auth/verify-key"))
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

/// 验证玩家登录凭据，返回 (username, token)
pub fn validate_credentials(api_key: &str, creds: &AuthCredentials) -> Result<(String, String), String> {
    let body = serde_json::to_string(creds).map_err(|_| "Failed to serialize credentials".to_string())?;

    let response = tls_agent()
        .post(&platform_url("/api/auth/login"))
        .set("Authorization", &auth_header(api_key))
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| format!("认证服务不可用: {}", e))?;

    let auth_response: AuthResponse = response
        .into_json()
        .map_err(|e| format!("Invalid platform response: {}", e))?;

    if auth_response.success {
        Ok((auth_response.username, auth_response.token))
    } else {
        Err(auth_response.message)
    }
}

/// 续约 session
pub fn renew_session(api_key: &str, token: &str) -> Result<(), String> {
    let body = serde_json::to_string(&RenewRequest { token: token.to_string() })
        .map_err(|_| "Failed to serialize renew request".to_string())?;

    let response = tls_agent()
        .post(&platform_url("/api/session/renew"))
        .set("Authorization", &auth_header(api_key))
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| format!("Session renew service unavailable: {}", e))?;

    let renew_response: RenewResponse = response
        .into_json()
        .map_err(|e| format!("Invalid platform renew response: {}", e))?;

    if renew_response.success {
        Ok(())
    } else {
        Err(renew_response.message)
    }
}
