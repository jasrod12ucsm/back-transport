// use std::sync::Arc;
//
// use ac_struct_back::schemas::auth::{
//     user::user::UserConfigError, user_email_verify::user_email_verify::UpdateEmailVerifyError,
// };
// use common::utils::ntex_private::extractors::json::JsonAdvanced;
// use ntex::web::{
//     self,
//     types::{Path, State},
// };
//
// use crate::modules::user_management::domain::{
//     cases::{
//         generate_code_use_case::{GenerateCodeUseCase, GenerateCodeUseCaseTrait},
//         register_use_case::{RegisterUseCase, RegisterUseCaseTrait},
//         verify_code_use_case::{VerifyCodeUseCase, VerifyCodeUseCaseTrait},
//     },
//     data::{
//         generate_code_dto::GenerateCodeDto, generate_code_response::GenerateCodeResponse,
//         register_dto::RegisterDto, register_response::RegisterResponse,
//     },
//     models::send_email_model::SendEmailModel,
// };
//
// #[web::post("register")]
// async fn register_user(
//     user: common::utils::ntex_private::extractors::json::JsonAdvanced<RegisterDto>,
//     tx: State<Arc<SendEmailModel>>,
// ) -> Result<JsonAdvanced<RegisterResponse>, UserConfigError> {
//     RegisterUseCase::new()
//         .register_user(user.into_inner(), &tx)
//         .await
// }
//
// #[web::post("generate_code")]
// async fn generate_code(
//     user: common::utils::ntex_private::extractors::json::JsonAdvanced<GenerateCodeDto>,
//     tx: State<Arc<SendEmailModel>>,
// ) -> Result<JsonAdvanced<GenerateCodeResponse>, UpdateEmailVerifyError> {
//     let gn_code = GenerateCodeUseCase {};
//     gn_code.execute(user.0, &tx).await
// }
//
// #[web::get("verify_code/{code}/{email}")]
// async fn verify_code(code_and_mail: Path<(String, String)>) -> Result<String, UserConfigError> {
//     let (code, email) = code_and_mail.into_inner();
//     let vcode = VerifyCodeUseCase {};
//     vcode.verify_code(&code, &email).await
// }
//
// // #[ntex::test]
// // async fn test_register_user() {
// //     let req = ntex::web::test::TestRequest::get()
// //         .uri("/")
// //         .to_http_request();
// //     let rest = ntex::web::test::respond_to(hello(), &req).await;
// //     assert_eq!(rest.status(), ntex::http::StatusCode::OK);
// //     let body = ntex::web::test::read_body(rest).await;
// //     assert_eq!(body, b"Hello, World!");
// // }
