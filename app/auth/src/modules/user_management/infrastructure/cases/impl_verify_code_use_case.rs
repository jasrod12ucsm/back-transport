// use std::{borrow::Cow, time::Duration};
//
// use ac_struct_back::{
//     schemas::auth::{
//         user::user::{UserConfig, UserConfigError, userconfigiduserconfig::UserConfigId},
//         user_email_verify::user_email_verify::{
//             UserEmailVerify, useremailverifyiduseremailverify::UserEmailVerifyId,
//         },
//     },
//     utils::domain::query::{
//         Condition, OneOrMany, Operator, Query, ReturnClause, UpdateRequest, UpdateTarget,
//         comparison, execute_select_query, execute_update_query, record_id_comparison,
//     },
// };
// use chrono::Utc;
// use ntex::http::StatusCode;
// use serde_json::Value;
// use smallvec::smallvec;
// use surrealdb::{Surreal, engine::any::Any};
//
// use crate::{
//     modules::user_management::domain::cases::verify_code_use_case::{
//         VerifyCodeUseCase, VerifyCodeUseCaseTrait,
//     },
//     try_get_surreal_pool,
// };
//
// #[async_trait::async_trait]
// impl VerifyCodeUseCaseTrait for VerifyCodeUseCase {
//     async fn verify_code(&self, code: &str, email: &str) -> Result<String, UserConfigError> {
//         println!("mail: {:?}", email);
//         println!("code: {:?}", code);
//         let conn = try_get_surreal_pool()
//             .ok_or_else(|| UserConfigError {
//                 message: "SurrealDB connection pool not initialized".to_string(),
//                 status_code: StatusCode::INTERNAL_SERVER_ERROR,
//             })?
//             .get()
//             .await
//             .map_err(|e| UserConfigError {
//                 message: format!("Failed to get SurrealDB connection: {}", e),
//                 status_code: StatusCode::INTERNAL_SERVER_ERROR,
//             })?;
//         let code = self
//             .verify_code(&code, &email, &conn.as_ref().client)
//             .await?;
//         Ok(code.to_string())
//     }
// }
//
// impl VerifyCodeUseCase {
//     //* Paso 1 Unico paso
//     //traer la data de la tabla de mst_user_email_verify
//     async fn verify_code(
//         &self,
//         code: &str,
//         email: &str,
//         db: &Surreal<Any>,
//     ) -> Result<String, UserConfigError> {
//         let now_less_1_min_30_secs = Utc::now() - Duration::from_secs(120);
//         let email_validation: OneOrMany<UserConfigId> = execute_select_query(
//             Query::<UserConfig>::new()
//                 .from(None, true)
//                 .condition(comparison(
//                     "email",
//                     ac_struct_back::utils::domain::query::Operator::Eq,
//                     "mail",
//                 ))
//                 .parameter("mail", Value::from(email))
//                 .get_owned(),
//             db,
//             true,
//         )
//         .await?;
//         let user_id = if let OneOrMany::One(val) = email_validation {
//             if val.is_none() {
//                 return Err(UserConfigError {
//                     message: "Email not found".to_string(),
//                     status_code: StatusCode::NOT_FOUND,
//                 });
//             } else {
//                 let val = val.unwrap();
//                 if val.is_verified {
//                     return Err(UserConfigError {
//                         message: "Email already verified".to_string(),
//                         status_code: StatusCode::NOT_FOUND,
//                     });
//                 } else {
//                     val.id.unwrap()
//                 }
//             }
//         } else {
//             return Err(UserConfigError {
//                 message: "Email not found".to_string(),
//                 status_code: StatusCode::NOT_FOUND,
//             });
//         };
//
//         let find: OneOrMany<UserEmailVerifyId> = execute_select_query(
//             Query::<UserEmailVerify>::new()
//                 .from(None, true)
//                 .condition(comparison(
//                     "verify_code",
//                     ac_struct_back::utils::domain::query::Operator::Eq,
//                     "c",
//                 ))
//                 .condition(comparison(
//                     "last_verify_code_at",
//                     ac_struct_back::utils::domain::query::Operator::Gte,
//                     "now_less_1_30_min",
//                 ))
//                 .condition(record_id_comparison("user_id", Operator::Eq, "user_id"))
//                 .parameter(
//                     "now_less_1_30_min",
//                     serde_json::to_value(now_less_1_min_30_secs).map_err(|_| UserConfigError {
//                         message: "Failed send request".to_string(),
//                         status_code: StatusCode::INTERNAL_SERVER_ERROR,
//                     })?,
//                 )
//                 .parameter("c", Value::from(code))
//                 .parameter("user_id", Value::from(user_id.to_string().as_str()))
//                 .fields(&["id"])
//                 .get_owned(),
//             db,
//             true,
//         )
//         .await?;
//
//         if let OneOrMany::One(val) = find {
//             if val.is_none() {
//                 return Err(UserConfigError {
//                     message: "Code not found".to_string(),
//                     status_code: StatusCode::NOT_FOUND,
//                 });
//             } else {
//                 //actualizar el codigo en la tabla de mst_user_config_is_verified
//                 let update_query: Vec<UserConfigId> = execute_update_query(
//                     UpdateRequest::<UserConfig>::builder()
//                         .update(Some(UpdateTarget::Subquery(
//                             Query::<UserConfig>::new()
//                                 .from(Some(user_id.key().to_string().as_str()), false)
//                                 .get_owned(),
//                         )))
//                         .map_err(|_| UserConfigError {
//                             message: "Failed send request".to_string(),
//                             status_code: StatusCode::INTERNAL_SERVER_ERROR,
//                         })?
//                         .set("is_verified", "isverified")
//                         .parameter("isverified", Value::from(true))
//                         .return_clause(ReturnClause::Fields(smallvec![
//                             Cow::Borrowed("id"),
//                             Cow::Borrowed("is_verified"),
//                         ]))
//                         .get_owned(),
//                     db,
//                     false,
//                 )
//                 .await
//                 .map_err(|_: UserConfigError| UserConfigError {
//                     message: format!("Failed to update code"),
//                     status_code: StatusCode::INTERNAL_SERVER_ERROR,
//                 })?;
//                 if update_query.len() != 1 {
//                     return Err(UserConfigError {
//                         message: "Failed to update code".to_string(),
//                         status_code: StatusCode::INTERNAL_SERVER_ERROR,
//                     });
//                 }
//
//                 return Ok(
//                     "code verificado, inicie session para empezar la experiencia".to_string(),
//                 );
//             }
//         } else {
//             return Err(UserConfigError {
//                 message: "Code not found".to_string(),
//                 status_code: StatusCode::NOT_FOUND,
//             });
//         }
//     }
// }
