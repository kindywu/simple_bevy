use crate::shared::{AuthCredentials, AuthResponse, PLATFORM_PORT};
use bevy::prelude::*;

#[derive(Resource)]
pub struct ApiKey(pub String);

pub fn validate_credentials(api_key: &str, creds: &AuthCredentials) -> Result<String, String> {
    let url = format!("http://127.0.0.1:{}/api/auth", PLATFORM_PORT);
    let body = serde_json::to_string(creds).map_err(|_| "Failed to serialize credentials".to_string())?;

    let response = ureq::post(&url)
        .set("Authorization", &format!("Bearer {}", api_key))
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| format!("Platform unreachable: {}", e))?;

    let auth_response: AuthResponse = response
        .into_json()
        .map_err(|e| format!("Invalid platform response: {}", e))?;

    if auth_response.success {
        Ok(auth_response.username)
    } else {
        Err(auth_response.message)
    }
}
