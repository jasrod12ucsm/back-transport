use surrealdb::RecordId;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CountryId {
    pub id: Option<RecordId>,
}
