use ac_struct_back::schemas::auth::user::user::UserConfigError;
use common::utils::ntex_private::extractors::json::JsonAdvanced;

use crate::modules::auth::{
    domain::data::{user_login_dto::UserLoginDto, user_login_response::UserLoginResponse},
    infrastructure::use_case::impl_login_use_case::MyHttpRequest,
};

pub struct LoginUseCase;
#[async_trait::async_trait]
pub trait LoginUseCaseTrait {
    async fn execute(
        user_login_dto: UserLoginDto,
        req: MyHttpRequest,
    ) -> Result<JsonAdvanced<UserLoginResponse>, UserConfigError>;
}
