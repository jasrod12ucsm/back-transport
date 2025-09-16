use ac_struct_back::schemas::config::feature::feature_type::FeatureType;
use surrealdb::RecordId;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeaturesForProyect {
    pub id: RecordId,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_feature: Option<FeatureType>,
}
