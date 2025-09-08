use surrealdb::RecordId;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct TimezoneId {
    pub id: Option<RecordId>,
    pub name: String,
}
