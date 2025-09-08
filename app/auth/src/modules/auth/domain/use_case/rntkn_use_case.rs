use ac_struct_back::schemas::auth::user::user::UserConfigError;
use common::utils::ntex_private::extractors::json::JsonAdvanced;

use crate::modules::auth::{
    domain::data::user_login_response::UserLoginResponse,
    infrastructure::use_case::impl_login_use_case::MyHttpRequest,
};

pub struct RenewTokenUseCase;

#[async_trait::async_trait]
pub trait RenewTokenUseCaseTrait {
    async fn execute(
        req: MyHttpRequest,
    ) -> Result<JsonAdvanced<UserLoginResponse>, UserConfigError>;
}
