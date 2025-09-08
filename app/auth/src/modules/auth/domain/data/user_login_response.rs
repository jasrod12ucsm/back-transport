use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserLoginResponse {
    pub token: String,
    pub refresh_token: String,
}

impl UserLoginResponse {
    pub fn new(token: String, refresh_token: String) -> Self {
        Self {
            token,
            refresh_token,
        }
    }
}
