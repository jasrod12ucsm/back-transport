use validator::Validate;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct VoidStruct {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,
}

impl Default for VoidStruct {
    fn default() -> Self {
        VoidStruct { separator: None }
    }
}
