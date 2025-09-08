#[derive(Debug, Clone, serde::Deserialize)]
pub struct EnvConfig {
    pub SECRET: String,
    pub API_PORT: u16,
    pub API_IP: String,
    pub ALLOWED_ORIGIN: String,
    pub SUR_USERNAME: String,
    pub SUR_PASSWORD: String,
    pub PUBLIC_SUR_USERNAME: String,
    pub PUBLIC_SUR_PASSWORD: String,
    pub SURREAL_URL: String,
    pub EXP_MINUTES_SURREAL: u64,
}
