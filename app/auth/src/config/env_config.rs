#[derive(Debug, Clone, serde::Deserialize)]
pub struct EnvConfig {
    pub SECRET: String,
    pub AUTH_PORT: u16,
    pub AUTH_IP: String,
    pub ALLOWED_ORIGIN: String,
    pub SMTP_EMAIL: String,
    pub SMTP_SERVER: String,
    pub SMTP_PASSWORD: String,
    pub SUR_USERNAME: String,
    pub SUR_PASSWORD: String,
    pub PUBLIC_SUR_USERNAME: String,
    pub PUBLIC_SUR_PASSWORD: String,
    pub SURREAL_URL: String,
    pub EXP_MINUTES_SURREAL: u64,
}
