mod auth;
mod db;
mod handlers;
mod models;

use crate::models::AppState;
use anyhow::{Context, Result};
use axum::{
    Router,
    routing::{get, post},
};
use axum_server::tls_rustls::RustlsConfig;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[tokio::main]
async fn main() -> Result<()> {
    let players_path = format!("{MANIFEST_DIR}/players.json");
    let api_keys_path = format!("{MANIFEST_DIR}/api_keys.json");

    let players = db::load_players(&players_path).context("加载玩家数据失败")?;
    let api_keys = db::load_api_keys(&api_keys_path).context("加载 API Key 失败")?;

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
        .route("/api/auth/verify-key", post(handlers::verify_key_handler))
        .route("/api/auth/login", post(handlers::login_handler))
        .route("/api/session/renew", post(handlers::renew_handler))
        .route("/api/health", get(handlers::health_handler))
        .with_state(state);

    let tls_config = RustlsConfig::from_pem_file(
        format!("{MANIFEST_DIR}/certs/localhost.pem"),
        format!("{MANIFEST_DIR}/certs/localhost-key.pem"),
    )
    .await
    .context("加载 TLS 证书失败 (请先运行: mkcert -install && cd platform/certs && mkcert localhost 127.0.0.1 ::1)")?;

    let addr = "127.0.0.1:3001".parse().context("解析监听地址失败")?;
    println!("Platform listening on https://127.0.0.1:3001");

    axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await
        .context("启动 HTTPS 服务失败")?;

    Ok(())
}
