// use ac_struct_back::{
//     import::macro_import::TableName,
//     schemas::auth::{
//         user::user::UserConfig,
//         user_email_verify::user_email_verify::{UpdateEmailVerifyError, UserEmailVerify},
//     },
//     utils::domain::query::{
//         OneOrMany, Operator, Query, UpdateRequestBuilder, UpdateTarget, comparison,
//         execute_select_query, execute_update_query, record_id_comparison,
//     },
// };
// use chrono::Utc;
// use common::{
//     public::functions::random_code::RandomCodeGenerator,
//     utils::ntex_private::extractors::json::JsonAdvanced,
// };
// use ntex::http::StatusCode;
// use serde_json::Value;
// use surrealdb::{Surreal, engine::any::Any};
//
// use crate::{
//     modules::user_management::domain::{
//         cases::generate_code_use_case::{GenerateCodeUseCase, GenerateCodeUseCaseTrait},
//         data::{generate_code_dto::GenerateCodeDto, generate_code_response::GenerateCodeResponse},
//         models::{
//             send_email_model::{EmailMessage, SendEmailModel},
//             user_config_verified::UserConfigVerified,
//         },
//     },
//     try_get_surreal_pool,
// };
//
// #[async_trait::async_trait]
// impl GenerateCodeUseCaseTrait for GenerateCodeUseCase {
//     async fn execute(
//         &self,
//         input: GenerateCodeDto,
//         tx: &SendEmailModel,
//     ) -> Result<JsonAdvanced<GenerateCodeResponse>, UpdateEmailVerifyError> {
//         let conn = try_get_surreal_pool()
//             .ok_or_else(|| UpdateEmailVerifyError {
//                 message: "SurrealDB connection pool not initialized".to_string(),
//                 status_code: StatusCode::INTERNAL_SERVER_ERROR,
//             })?
//             .get()
//             .await
//             .map_err(|e| UpdateEmailVerifyError {
//                 message: format!("Failed to get SurrealDB connection: {}", e),
//                 status_code: StatusCode::INTERNAL_SERVER_ERROR,
//             })?;
//
//         if !input.validate_email() {
//             return Err(UpdateEmailVerifyError {
//                 message: "Email invalido".to_string(),
//                 status_code: StatusCode::BAD_REQUEST,
//             });
//         }
//         // Validar que el usuario exista y no esté verificado
//         let val = self
//             .validate_user_exist(&input.email, &conn.as_ref().client)
//             .await?;
//         // Generar el código aleatorio
//         let code = self.generate_verify_code().await?;
//         // Guardar el código en la base de datos
//         self.update_code(&conn.as_ref().client, &val, code.as_str())
//             .await?;
//         // enviar email
//         self.send_email(&tx, &input.email, &code, &conn.as_ref().client)
//             .await?;
//
//         Ok(JsonAdvanced(GenerateCodeResponse {
//             message: "Codigo generado mire su correo".to_string(),
//         }))
//     }
// }
// impl GenerateCodeUseCase {
//     //* Generate code
//     async fn generate_verify_code(&self) -> Result<String, UpdateEmailVerifyError> {
//         let code =
//             RandomCodeGenerator::generate_unique_code().map_err(|e| UpdateEmailVerifyError {
//                 message: format!("Failed to generate verification code: {}", e),
//                 status_code: StatusCode::INTERNAL_SERVER_ERROR,
//             })?;
//         Ok(format!("{:06}", code)) // Formatear el código a 6 dígitos
//     }
//
//     //* This function is encharge of checking if the user exists and is not verified else return
//     //error
//     async fn validate_user_exist(
//         &self,
//         email: &str,
//         db: &Surreal<Any>,
//     ) -> Result<String, UpdateEmailVerifyError> {
//         //buscar si existe el usuario y no esta verificado
//         let user: OneOrMany<UserConfigVerified> = execute_select_query(
//             Query::<UserConfig>::new()
//                 .from(None, true)
//                 .condition(comparison("email", Operator::Eq, "email"))
//                 .condition(comparison("is_verified", Operator::Eq, "is_verified"))
//                 .fields(&["id", "is_verified"])
//                 .parameter("email", Value::from(email))
//                 .parameter("is_verified", Value::from(false))
//                 .get_owned(),
//             db,
//             true,
//         )
//         .await?;
//
//         if let OneOrMany::One(val) = user {
//             if val.is_none() {
//                 return Err(UpdateEmailVerifyError {
//                     message: "User not found".to_string(),
//                     status_code: StatusCode::NOT_FOUND,
//                 });
//             } else {
//                 return Ok(val.unwrap().id.unwrap().key().to_string());
//             }
//         } else {
//             return Err(UpdateEmailVerifyError {
//                 message: "User not found".to_string(),
//                 status_code: StatusCode::NOT_FOUND,
//             });
//         }
//     }
//     //* rhis function update the code generated
//     async fn update_code<'a>(
//         &self,
//         db: &'a Surreal<Any>,
//         user_key: &str,
//         code: &str,
//     ) -> Result<String, UpdateEmailVerifyError> {
//         let query = Query::<UserEmailVerify>::new()
//             .from(None, false)
//             .condition(record_id_comparison("user_id", Operator::Eq, "user_id"))
//             .get_owned();
//
//         let update_query: Vec<UserEmailVerify> = execute_update_query(
//             UpdateRequestBuilder::<UserEmailVerify>::new()
//                 .update(Some(UpdateTarget::<UserEmailVerify>::Subquery(query)))
//                 .map_err(|_| UpdateEmailVerifyError {
//                     message: "Failed send request".to_string(),
//                     status_code: StatusCode::INTERNAL_SERVER_ERROR,
//                 })?
//                 .set("verify_code", "code")
//                 .set("last_verify_code_at", "last_verify_code_at")
//                 .parameter("code", Value::from(code))
//                 .parameter(
//                     "user_id",
//                     serde_json::to_value(
//                         format!("{}:{}", UserConfig::table_name(), user_key).as_str(),
//                     )
//                     .map_err(|_| UpdateEmailVerifyError {
//                         message: "Failed send request".to_string(),
//                         status_code: StatusCode::INTERNAL_SERVER_ERROR,
//                     })?,
//                 ) //user_dto.id.as_ref().unwrap().key().to_string().as_str()))
//                 .parameter(
//                     "last_verify_code_at",
//                     serde_json::to_value(Utc::now()).map_err(|_| UpdateEmailVerifyError {
//                         message: "Failed send request".to_string(),
//                         status_code: StatusCode::INTERNAL_SERVER_ERROR,
//                     })?,
//                 )
//                 .get_owned(), //user_dto.id.as_ref().unwrap().key().to_string().as_str()))
//             db,
//             true,
//         )
//         .await?;
//         if update_query.len() != 1 {
//             return Err(UpdateEmailVerifyError {
//                 message: "Failed to update code".to_string(),
//                 status_code: StatusCode::INTERNAL_SERVER_ERROR,
//             });
//         }
//         Ok(update_query
//             .get(0)
//             .ok_or_else(|| UpdateEmailVerifyError {
//                 message: "Failed to update code".to_string(),
//                 status_code: StatusCode::INTERNAL_SERVER_ERROR,
//             })?
//             .verify_code
//             .as_ref()
//             .ok_or_else(|| UpdateEmailVerifyError {
//                 message: "Failed to update code".to_string(),
//                 status_code: StatusCode::INTERNAL_SERVER_ERROR,
//             })?
//             .clone())
//     }
//     //* This function send the email
//     async fn send_email<'a>(
//         &self,
//         tx: &'a SendEmailModel,
//         email: &str,
//         code: &str,
//         db: &'a Surreal<Any>,
//     ) -> Result<(), UpdateEmailVerifyError> {
//         tx.tx
//             .send(EmailMessage {
//                 code: code.to_string(),
//                 email: email.to_string(),
//                 db: db.clone(),
//             })
//             .await
//             .map_err(|_| UpdateEmailVerifyError {
//                 message: format!("Failed to send email"),
//                 status_code: StatusCode::INTERNAL_SERVER_ERROR,
//             })?;
//
//         Ok(())
//     }
// }
