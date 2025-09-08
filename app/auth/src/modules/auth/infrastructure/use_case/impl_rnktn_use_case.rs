use std::{borrow::Cow, fmt::format};

use ac_struct_back::{
    schemas::auth::user::{
        user::{UserConfig, UserConfigError},
        user_config_session::user_config_session::UserConfigSession,
    },
    utils::domain::query::{
        Condition, Function, OneOrMany, Operator, PatchOpType, PatchOperation, Query, ReturnClause,
        UpdateRequest, UpdateTarget, comparison, execute_select_query, execute_update_query,
    },
};
use chrono::{Duration, Utc};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::http::StatusCode;
use serde_json::Value;
use surrealdb::{RecordId, Surreal, engine::any::Any};

use crate::{
    modules::auth::domain::{
        data::user_login_response::UserLoginResponse,
        models::{renew_token_user_model::RenewTokenUserModel, user_config_id::UserConfigId},
        use_case::rntkn_use_case::{RenewTokenUseCase, RenewTokenUseCaseTrait},
    },
    try_get_surreal_pool,
    utils::infrastructure::functions::token::token_generator::{
        JwtGenerator, SECRET_REFRESH_TOKEN_BYTES, SECRET_TOKEN_BYTES,
    },
};

use super::impl_login_use_case::MyHttpRequest;
#[async_trait::async_trait]
impl RenewTokenUseCaseTrait for RenewTokenUseCase {
    async fn execute(
        req: MyHttpRequest,
    ) -> Result<JsonAdvanced<UserLoginResponse>, UserConfigError> {
        let conn = try_get_surreal_pool()
            .ok_or_else(|| UserConfigError {
                message: "SurrealDB connection pool not initialized".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .get()
            .await
            .map_err(|e| UserConfigError {
                message: format!("Failed to get SurrealDB connection: {}", e),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        let sub = Self::get_sub(&req).map_err(|e| UserConfigError {
            message: e,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })?;
        println!("sub: {:?}", sub);
        let fingerprint = Self::get_fingerprint(&req).map_err(|e| UserConfigError {
            message: e,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })?;
        let token = Self::get_token(&req).map_err(|e| UserConfigError {
            message: e,
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })?;
        //obtener la session
        let (session, refresh_token) =
            Self::get_session(token, fingerprint.clone(), sub.as_str(), &conn.client)
                .await
                .map_err(|e| UserConfigError {
                    message: e.message,
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })?;
        //generar token (no de refresh_token)
        let token = JwtGenerator::new_from_pem_bytes(SECRET_TOKEN_BYTES)
            .map_err(|_| UserConfigError {
                message: "Error al generar el token".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .generate_token(
                Some(session.key().to_string()),
                Some(fingerprint.to_string()),
                60 * 5,
            )
            .map_err(|_| UserConfigError {
                message: "Error al generar el token".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        let result = JsonAdvanced(UserLoginResponse {
            token: token,
            refresh_token: refresh_token,
        });
        Ok(result)
    }
}

impl RenewTokenUseCase {
    //* Esta funcion consigue el token de authorization request
    //
    //
    fn get_sub(req: &MyHttpRequest) -> Result<String, String> {
        req.0
            .headers()
            .get("i-sub")
            .ok_or_else(|| "Important Header not found".to_string())?
            .to_str()
            .map(|s| s.to_string())
            .map_err(|e| e.to_string())
    }
    fn get_token(req: &MyHttpRequest) -> Result<String, String> {
        req.0
            .headers()
            .get("Authorization")
            .ok_or_else(|| "Authorization header not found".to_string())?
            .to_str()
            .map(|s| {
                //split
                let parts: Vec<&str> = s.split(' ').collect();
                let token = parts.get(1);
                if token.is_none() {
                    return Err("Token not found".to_string());
                }
                Ok(token.unwrap().to_string())
            })
            .map_err(|e| e.to_string())?
    }
    //* Esta funcion se encarga de conseguir los fingerprint i-******** del header
    fn get_fingerprint(req: &MyHttpRequest) -> Result<String, String> {
        req.0
            .headers()
            .get("I-Fingerprint")
            .ok_or_else(|| "Header importante no encontrado".to_string())?
            .to_str()
            .map(|s| s.to_string())
            .map_err(|e| e.to_string())
    }
    //* esta funcion busca la session que coincida con el token y el fingerprint si no devuelve
    //error con el mensage de mandar de nuevo a login
    async fn get_session(
        token: String,
        finger_print: String,
        id: &str,
        db: &Surreal<Any>,
    ) -> Result<(RecordId, String), UserConfigError> {
        let mut session: OneOrMany<RenewTokenUserModel> = execute_select_query(
            Query::<UserConfig>::new()
                .from(Some(id), true)
                .condition(Condition::Raw {
                    expression: Cow::Borrowed(" sessions.filter((|$v| v.fingerprint=$fp))"),
                    params: vec![],
                })
                .condition(comparison("deleted_at", Operator::Eq, "$del"))
                .fields(&["id", "sessions"])
                .parameter("del", Value::from(None::<String>))
                .parameter("rf", Value::from(token))
                .parameter("fp", Value::from(finger_print.clone()))
                .get_owned(),
            db,
            false,
        )
        .await?;
        //haz match de impresion

        match session {
            OneOrMany::One(Some(ref mut session)) => {
                if session.sessions.len() > 5 {
                    return Err(UserConfigError {
                        message: "Muchas sesiones activas, cierre alguna".to_string(),
                        status_code: StatusCode::UNAUTHORIZED,
                    });
                }
                println!("session: {:?}", session.clone());

                // Buscamos el índice primero, evitando mantener vivo el mutable borrow
                let maybe_index = {
                    session
                        .sessions
                        .iter()
                        .enumerate()
                        .find(|(_, s)| {
                            s.fingerprint == finger_print
                                && Utc::now().signed_duration_since(s.last_access)
                                    < Duration::days(3)
                        })
                        .map(|(i, _)| i)
                };

                if let Some(index) = maybe_index {
                    let elapsed =
                        Utc::now().signed_duration_since(session.sessions[index].last_access);

                    if elapsed > Duration::days(2) {
                        let new_token =
                            JwtGenerator::new_from_pem_bytes(SECRET_REFRESH_TOKEN_BYTES)
                                .map_err(|_| UserConfigError {
                                    message: "Error al generar el token de refresco".to_string(),
                                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                                })?
                                .generate_token(
                                    Some(session.id.key().to_string()),
                                    Some(finger_print.to_string()),
                                    60 * 60 * 24 * 3,
                                )
                                .map_err(|_| UserConfigError {
                                    message: "Error al generar el token de refresco".to_string(),
                                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                                })?;

                        // Mutamos la sesión entera después de liberar el borrow de .find
                        Self::update_session(session, &db, new_token.clone(), index).await?;

                        session.sessions[index].refresh_token = new_token;
                        session.sessions[index].last_access = Utc::now();
                    }

                    // Retornar referencias si posible, o valores movidos (sin clones)
                    return Ok((
                        session.id.clone(),
                        session.sessions[index].refresh_token.clone(),
                    ));
                } else {
                    return Err(UserConfigError {
                        message: "No session encontrada".to_string(),
                        status_code: StatusCode::UNAUTHORIZED,
                    });
                }
            }
            _ => {
                return Err(UserConfigError {
                    message: "No session encontrada".to_string(),
                    status_code: StatusCode::UNAUTHORIZED,
                });
            }
        }
    }
    //* Esta cuncion actualiza el refresh token y el last_access
    async fn update_session(
        session: &mut RenewTokenUserModel,
        db: &Surreal<Any>,
        new_token: String,
        index: usize,
    ) -> Result<(), UserConfigError> {
        use smallvec::smallvec;
        let now_access = Utc::now();
        let update_response: Vec<UserConfigId> = execute_update_query(
            UpdateRequest::<UserConfig>::builder()
                .update(Some(UpdateTarget::Subquery(
                    Query::<UserConfig>::new()
                        .from(Some(session.id.key().to_string().as_str()), false)
                        .get_owned(),
                )))
                .map_err(|e| UserConfigError {
                    message: format!("Failed to build update query: {}", e),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })? //user_dto.id.as_ref().unwrap().key().to_string().as_str()))
                .patch(PatchOperation {
                    path: format!("sessions/{}/refresh_token", index),
                    op: PatchOpType::Replace,
                    value: serde_json::to_value(new_token).map_err(|e| UserConfigError {
                        message: format!("Error al Tratar la data"),
                        status_code: StatusCode::INTERNAL_SERVER_ERROR,
                    })?,
                })
                .patch(PatchOperation {
                    path: format!("sessions/{}/last_access", index),
                    op: PatchOpType::Replace,
                    value: serde_json::to_value(now_access).map_err(|e| UserConfigError {
                        message: format!("Error al Tratar la data"),
                        status_code: StatusCode::INTERNAL_SERVER_ERROR,
                    })?,
                })
                .return_clause(ReturnClause::Fields(smallvec![Cow::Borrowed("id")]))
                .get_owned(),
            db,
            false,
        )
        .await?;
        if update_response.len() != 1 {
            return Err(UserConfigError {
                message: "No se guardo el código de verificación".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
        Ok(())
    }
}
