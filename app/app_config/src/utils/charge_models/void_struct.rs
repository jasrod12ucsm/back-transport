use validator::Validate;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Validate)]
pub struct VoidStruct {}

impl Default for VoidStruct {
    fn default() -> Self {
        Self {}
    }
}
