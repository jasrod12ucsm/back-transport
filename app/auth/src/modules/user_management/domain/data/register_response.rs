#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct RegisterResponse {
    pub verify_code: String,
}
