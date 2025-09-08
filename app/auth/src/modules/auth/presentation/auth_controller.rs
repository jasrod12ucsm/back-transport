use ac_struct_back::schemas::auth::user::user::UserConfigError;
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::web;

use crate::modules::auth::{
    domain::{
        data::{user_login_dto::UserLoginDto, user_login_response::UserLoginResponse},
        use_case::{
            login_use_case::{LoginUseCase, LoginUseCaseTrait},
            rntkn_use_case::{RenewTokenUseCase, RenewTokenUseCaseTrait},
        },
    },
    infrastructure::use_case::impl_login_use_case::MyHttpRequest,
};

#[web::post("login")]
async fn login(
    path: JsonAdvanced<UserLoginDto>,
    req: web::HttpRequest,
) -> Result<JsonAdvanced<UserLoginResponse>, UserConfigError> {
    LoginUseCase::execute(path.0, MyHttpRequest(req)).await
}

pub async fn rntkn(
    req: web::HttpRequest,
) -> Result<JsonAdvanced<UserLoginResponse>, UserConfigError> {
    RenewTokenUseCase::execute(MyHttpRequest(req)).await
}
