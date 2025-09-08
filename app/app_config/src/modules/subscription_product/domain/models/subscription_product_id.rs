use surrealdb::RecordId;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubscriptionProductId {
    pub id: RecordId,
}
