use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateCodeDto {
    pub email: String,
}

impl GenerateCodeDto {
    pub fn validate_email(&self) -> bool {
        if self.email.is_empty() {
            return false;
        }
        if !self.email.contains('@') || !checkmail::validate_email(&self.email) {
            return false;
        }
        true
    }
}
