use ac_struct_back::schemas::config::feature::feature_type::FeatureType;
use validator::Validate;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, Validate)]
pub struct ChargeDto {
    pub fields: Vec<Field>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub separator: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Field {
    pub name: String,
    pub _type: FeatureType,
}
