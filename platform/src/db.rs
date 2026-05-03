use crate::auth::hash_password;
use crate::models::{ApiKeyDb, Player, PlayerDb};
use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};

pub type DbPool = Pool<Sqlite>;

pub async fn init_db(path: &str) -> Result<DbPool> {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .connect(&format!("sqlite:{path}"))
        .await
        .context("连接数据库失败")?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS players (
            username TEXT PRIMARY KEY,
            password_hash TEXT NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .context("创建 players 表失败")?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS api_keys (
            key TEXT PRIMARY KEY
        )
        "#,
    )
    .execute(&pool)
    .await
    .context("创建 api_keys 表失败")?;

    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM players")
        .fetch_one(&pool)
        .await
        .context("查询玩家数量失败")?;

    if count.0 == 0 {
        let default_players = ["kindy", "ananda", "martin", "amy"];
        for name in default_players {
            let hash = hash_password(name);
            sqlx::query("INSERT INTO players (username, password_hash) VALUES (?, ?)")
                .bind(name)
                .bind(&hash)
                .execute(&pool)
                .await
                .context("插入默认玩家失败")?;
        }
        println!("已创建默认玩家: kindy, ananda, martin, amy (密码同用户名)");
    }

    let key_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM api_keys")
        .fetch_one(&pool)
        .await
        .context("查询 API Key 数量失败")?;

    if key_count.0 == 0 {
        let default_key = "super-secret-platform-api-key";
        sqlx::query("INSERT INTO api_keys (key) VALUES (?)")
            .bind(default_key)
            .execute(&pool)
            .await
            .context("插入默认 API Key 失败")?;
        println!("已创建默认 API Key: {default_key}");
    }

    Ok(pool)
}

pub async fn load_players(pool: &DbPool) -> Result<PlayerDb> {
    let players: Vec<Player> = sqlx::query_as("SELECT username, password_hash FROM players")
        .fetch_all(pool)
        .await
        .context("查询玩家失败")?;
    Ok(PlayerDb { players })
}

pub async fn load_api_keys(pool: &DbPool) -> Result<ApiKeyDb> {
    let keys: Vec<(String,)> = sqlx::query_as("SELECT key FROM api_keys")
        .fetch_all(pool)
        .await
        .context("查询 API Keys 失败")?;
    let keys = keys.into_iter().map(|(k,)| k).collect();
    Ok(ApiKeyDb { keys })
}
