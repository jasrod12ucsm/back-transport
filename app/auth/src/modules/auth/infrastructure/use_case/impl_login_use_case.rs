use std::borrow::Cow;

use ac_struct_back::{
    schemas::auth::user::{
        user::{UserConfig, UserConfigError},
        user_config_session::{
            finger_print::finger_print::Fingerprint, user_config_session::UserConfigSession,
        },
    },
    utils::domain::query::{
        Condition, OneOrMany, Operator, PatchOpType, PatchOperation, Query, ReturnClause,
        UpdateRequest, UpdateTarget, comparison, execute_select_query, execute_update_query,
    },
};
use chrono::Utc;
use common::{
    helpers::password::password_functions::PasswordFunctions,
    utils::ntex_private::extractors::json::JsonAdvanced,
};
use ntex::{http::StatusCode, web::HttpRequest};
use serde_json::Value;
use surrealdb::{RecordId, Surreal, engine::any::Any};

use crate::{
    modules::{
        auth::domain::{
            data::{user_login_dto::UserLoginDto, user_login_response::UserLoginResponse},
            models::{user_config_id::UserConfigId, user_login_model::UserForLoginModel},
            use_case::login_use_case::{LoginUseCase, LoginUseCaseTrait},
        },
        user_management::domain::cases::register_use_case::{
            RegisterUseCase, RegisterUseCasePrivate,
        },
    },
    try_get_surreal_pool,
    utils::infrastructure::functions::token::token_generator::{
        JwtGenerator, SECRET_REFRESH_TOKEN_BYTES, SECRET_TOKEN_BYTES,
    },
};
pub struct MyHttpRequest(pub ntex::web::HttpRequest);
unsafe impl Send for MyHttpRequest {}
unsafe impl Sync for MyHttpRequest {}
#[async_trait::async_trait]
impl LoginUseCaseTrait for LoginUseCase {
    async fn execute(
        user_login_dto: UserLoginDto,
        req: MyHttpRequest,
    ) -> Result<JsonAdvanced<UserLoginResponse>, UserConfigError> {
        //verify password
        for (key, value) in req.0.headers().iter() {
            println!("key: {:?}, value: {:?}", key, value);
        }
        let finger_p = req.0.headers().get("I-Fingerprint");
        if finger_p.is_none() {
            return Err(UserConfigError {
                message: "No autorizado".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            });
        }
        let finger_print = finger_p.unwrap();
        let finger_print = finger_print.to_str().map_err(|_| UserConfigError {
            message: "No autorizado".to_string(),
            status_code: StatusCode::UNAUTHORIZED,
        })?;
        println!("finger_print: {:?}", finger_print);
        let cn_connecting_ip = req
            .0
            .headers()
            .get("i-cf-connecting-ip")
            .map(|v| v.to_str().map(|s| s.to_string()))
            .transpose()
            .map_err(|_| UserConfigError {
                message: "No Autorizado".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            })?;

        let cn_ipcountry = req
            .0
            .headers()
            .get("i-cf-ipcountry")
            .map(|v| v.to_str().map(|s| s.to_string()))
            .transpose()
            .map_err(|_| UserConfigError {
                message: "No Autorizado".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            })?;

        let user_agent = req
            .0
            .headers()
            .get("i-user-agent")
            .map(|v| v.to_str().map(|s| s.to_string()))
            .transpose()
            .map_err(|_| UserConfigError {
                message: "No Autorizado".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            })?;

        let sec_ch_ua_platform = req
            .0
            .headers()
            .get("i-sec-ch-ua-platform")
            .map(|v| v.to_str().map(|s| s.to_string()))
            .transpose()
            .map_err(|_| UserConfigError {
                message: "Invalid sec-ch-ua-platform format".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            })?;

        let sec_ch_ua = req
            .0
            .headers()
            .get("i-sec-ch-ua")
            .map(|v| v.to_str().map(|s| s.to_string()))
            .transpose()
            .map_err(|_| UserConfigError {
                message: "Invalid sec-ch-ua format".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            })?;
        //user agent es Requerido
        if cn_connecting_ip.is_none() || cn_connecting_ip.is_none() {
            return Err(UserConfigError {
                message: "Invalid cf-connecting-ip".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            });
        }
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
        let password = RegisterUseCase::validate_password(&user_login_dto.password)?;
        let user_password = Self::get_user_by_email(user_login_dto, &conn.client).await?;
        //validate password argon2
        let password_verified = PasswordFunctions::verify_password(&user_password.hash, &password)
            .map_err(|_| UserConfigError {
                message: format!("Failed to verify password"),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        if !password_verified {
            return Err(UserConfigError {
                message: "Password incorrect".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            });
        }
        //sessions
        let exist = Self::exist_session(&user_password.sessions, &finger_print).await?;
        let last_access = Utc::now();
        let refresh_token = JwtGenerator::new_from_pem_bytes(SECRET_REFRESH_TOKEN_BYTES)
            .map_err(|_| UserConfigError {
                message: "Error al generar el token de refresco".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .generate_token(
                Some(user_password.id.key().to_string()),
                Some(finger_print.to_string()),
                60 * 60 * 24 * 3,
            )
            .map_err(|_| UserConfigError {
                message: "Error al generar el token de refresco".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        let token = JwtGenerator::new_from_pem_bytes(SECRET_TOKEN_BYTES)
            .map_err(|_| UserConfigError {
                message: "Error al generar el token".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .generate_token(
                Some(user_password.id.key().to_string()),
                Some(finger_print.to_string()),
                60 * 5,
            )
            .map_err(|_| UserConfigError {
                message: "Error al generar el token".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;

        let session = UserConfigSession {
            fingerprint: finger_print.to_string(),
            des_fingerprint: Fingerprint {
                cf_connecting_ip: cn_connecting_ip,
                cf_ipcountry: cn_ipcountry,
                user_agent: user_agent,
                sec_ch_ua_platform: sec_ch_ua_platform,
                sec_ch_ua: sec_ch_ua,
            },
            refresh_token: refresh_token.clone(),
            last_access: last_access,
        };

        if exist < 0 {
            Self::add_new_session(&user_password, session, &conn.client).await?;
        } else {
            //update access
            let _ = Self::update_access_with_session(exist, session, &user_password, &conn.client)
                .await?;
        }
        //*
        Ok(JsonAdvanced(UserLoginResponse {
            token: token,
            refresh_token: refresh_token,
        }))
    }
}
impl LoginUseCase {
    //* obtener el user con el email, el usuario debe estar verificado
    async fn get_user_by_email(
        user_login_dto: UserLoginDto,
        db: &Surreal<Any>,
    ) -> Result<UserForLoginModel, UserConfigError> {
        let user_exist: OneOrMany<UserForLoginModel> = execute_select_query(
            Query::<UserConfig>::new()
                .from(None, true)
                .condition(comparison("email", Operator::Eq, "$mail"))
                .condition(comparison("is_verified", Operator::Eq, "$v"))
                .condition(comparison("deleted_at", Operator::Eq, "$del"))
                .parameter("del", Value::from(None::<String>))
                .parameter("mail", Value::from(user_login_dto.email))
                .parameter("v", Value::from(true))
                .fields(&["id", "hash", "sessions"])
                .get_owned(),
            db,
            true,
        )
        .await?;
        match user_exist {
            OneOrMany::One(user) => {
                if user.is_some() {
                    return Ok(user.unwrap());
                } else {
                    return Err(UserConfigError {
                        message: "Usuario no encontrado o no registrado".to_string(),
                        status_code: StatusCode::UNAUTHORIZED,
                    });
                }
            }
            OneOrMany::Many(_) => {
                return Err(UserConfigError {
                    message: "Usuario no encontrado o no registrado".to_string(),
                    status_code: StatusCode::UNAUTHORIZED,
                });
            }
        }
    }
    async fn exist_session(
        sessions: &Vec<UserConfigSession>,
        finger_print: &str,
    ) -> Result<i32, UserConfigError> {
        let mut index = -1i32;
        for (i, session) in sessions.iter().enumerate().rev() {
            if session.fingerprint == finger_print {
                index = i as i32;
                break;
            }
        }

        Ok(index)
    }

    async fn add_new_session(
        model: &UserForLoginModel,
        session: UserConfigSession,
        db: &Surreal<Any>,
    ) -> Result<(), UserConfigError> {
        use smallvec::smallvec;
        let update_response: Vec<UserConfigId> = execute_update_query(
            UpdateRequest::<UserConfig>::builder()
                .update(Some(UpdateTarget::Subquery(
                    Query::<UserConfig>::new()
                        .from(Some(model.id.key().to_string().as_str()), false)
                        .get_owned(),
                )))
                .map_err(|e| UserConfigError {
                    message: format!("Failed to build update query: {}", e),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })? //user_dto.id.as_ref().unwrap().key().to_string().as_str()))
                .patch(PatchOperation {
                    path: "/sessions/-".to_string(),
                    op: PatchOpType::Add,
                    value: serde_json::to_value(session).map_err(|e| UserConfigError {
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
                message: "Failed to save verification code".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
        Ok(())
    }
    async fn update_access_with_session(
        index: i32,
        session: UserConfigSession,
        model: &UserForLoginModel,
        db: &Surreal<Any>,
    ) -> Result<(), UserConfigError> {
        use smallvec::smallvec;
        let update_response: Vec<UserConfigId> = execute_update_query(
            UpdateRequest::<UserConfig>::builder()
                .update(Some(UpdateTarget::Subquery(
                    Query::<UserConfig>::new()
                        .from(Some(model.id.key().to_string().as_str()), false)
                        .get_owned(),
                )))
                .map_err(|e| UserConfigError {
                    message: format!("Failed to build update query: {}", e),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })? //user_dto.id.as_ref().unwrap().key().to_string().as_str()))
                .patch(PatchOperation {
                    path: format!("/sessions/{}", index),
                    op: PatchOpType::Replace,
                    value: serde_json::to_value(session).map_err(|e| UserConfigError {
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
                message: "Failed to save verification code".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
        Ok(())
    }
}
