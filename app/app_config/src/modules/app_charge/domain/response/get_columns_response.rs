#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GetColumnsResponse {
    pub fields: Vec<String>,
}
