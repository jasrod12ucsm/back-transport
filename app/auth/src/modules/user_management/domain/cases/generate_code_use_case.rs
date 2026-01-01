// use common::utils::ntex_private::extractors::json::JsonAdvanced;
//
// use crate::modules::user_management::domain::{
//     data::{generate_code_dto::GenerateCodeDto, generate_code_response::GenerateCodeResponse},
//     models::send_email_model::SendEmailModel,
// };
// use ac_struct_back::schemas::auth::user_email_verify::user_email_verify::UpdateEmailVerifyError;
//
// pub struct GenerateCodeUseCase;
// #[async_trait::async_trait]
// pub trait GenerateCodeUseCaseTrait {
//     async fn execute(
//         &self,
//         input: GenerateCodeDto,
//         tx: &SendEmailModel,
//     ) -> Result<JsonAdvanced<GenerateCodeResponse>, UpdateEmailVerifyError>;
// }
