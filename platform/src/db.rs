use crate::auth::hash_password;
use crate::models::{ApiKeyDb, Player, PlayerDb};
use anyhow::{Context, Result};

pub fn load_players(path: &str) -> Result<PlayerDb> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).with_context(|| format!("读取玩家数据库失败: {}", path))?,
    };

    let mut db: PlayerDb = if contents.trim().is_empty() {
        PlayerDb { players: vec![] }
    } else {
        serde_json::from_str(&contents).with_context(|| format!("解析玩家数据库失败: {}", path))?
    };

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
        let json = serde_json::to_string_pretty(&db).context("序列化玩家数据库失败")?;
        std::fs::write(path, json).with_context(|| format!("写入玩家数据库失败: {}", path))?;
    }
    Ok(db)
}

pub fn load_api_keys(path: &str) -> Result<ApiKeyDb> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).with_context(|| format!("读取 API Key 数据库失败: {}", path))?,
    };

    let db: ApiKeyDb = if contents.trim().is_empty() {
        ApiKeyDb { keys: vec![] }
    } else {
        serde_json::from_str(&contents)
            .with_context(|| format!("解析 API Key 数据库失败: {}", path))?
    };
    Ok(db)
}
