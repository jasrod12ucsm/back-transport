use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserLoginDto {
    pub email: String,
    pub password: String,
}
