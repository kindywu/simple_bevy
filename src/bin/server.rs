fn load_env() -> String {
    let env_path = concat!(env!("CARGO_MANIFEST_DIR"), "/.env");
    if let Ok(contents) = std::fs::read_to_string(env_path) {
        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                if key == "PLATFORM_API_KEY" {
                    return value.to_string();
                }
            }
        }
    }
    simple::shared::PLATFORM_API_KEY.to_string()
}

fn main() {
    let api_key = std::env::var("PLATFORM_API_KEY").unwrap_or_else(|_| load_env());
    simple::run_server(&api_key);
}
