use ac_struct_back::schemas::auth::user::user_config_session::user_config_session::UserConfigSession;
use surrealdb::RecordId;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RenewTokenUserModel {
    pub id: RecordId,
    pub sessions: Vec<UserConfigSession>,
}
