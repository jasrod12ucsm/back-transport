#[derive(Debug, Clone, serde::Deserialize)]
pub struct EnvConfig {
    pub SECRET: String,
    pub AUTHORIZATION_PORT: u16,
    pub AUTHORIZATION_IP: String,
    pub ALLOWED_ORIGIN: String,
    pub SUR_USERNAME: String,
    pub SUR_PASSWORD: String,
    pub PUBLIC_SUR_USERNAME: String,
    pub PUBLIC_SUR_PASSWORD: String,
    pub SURREAL_URL: String,
}
