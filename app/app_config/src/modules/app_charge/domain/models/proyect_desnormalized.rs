use ac_struct_back::schemas::config::feature::feature::Feature;
use surrealdb::RecordId;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProyectDesnormalized {
    pub id: RecordId,
    pub fields: Vec<Feature>,
}
