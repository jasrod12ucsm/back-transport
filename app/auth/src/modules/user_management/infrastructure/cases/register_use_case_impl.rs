use smallvec::smallvec;
use std::{borrow::Cow, collections::HashMap, vec};

use ac_struct_back::{
    common::enums::status_enum::StatusEnum,
    import::macro_import::TableName,
    schemas::{
        auth::{
            user::user::{
                UserConfig, UserConfigError, registeruserdtouserconfig::RegisterUserDto,
                userconfigiduserconfig::UserConfigId,
            },
            user_email_verify::user_email_verify::{
                UserEmailVerify, useremailverifyiduseremailverify::UserEmailVerifyId,
            },
        },
        config::{
            template::template::Template,
            template_type::template_type::{
                TemplateType, templatetypeidtemplatetype::TemplateTypeId,
            },
        },
    },
    utils::domain::query::{
        Condition, OneOrMany, Operator, Query, ReturnClause, UpdateRequest, UpdateTarget,
        comparison, execute_select_query, execute_update_query, record_id_comparison,
    },
};
use chrono::{Datelike, Utc};
use common::{
    helpers::password::password_functions::PasswordFunctions,
    public::functions::random_code::RandomCodeGenerator,
    utils::ntex_private::extractors::json::JsonAdvanced,
};
use ntex::http::StatusCode;
use serde_json::Value;
use surrealdb::{RecordId, Surreal, engine::any::Any};

use crate::{
    modules::user_management::domain::{
        cases::register_use_case::{RegisterUseCase, RegisterUseCasePrivate, RegisterUseCaseTrait},
        data::{register_dto::RegisterDto, register_response::RegisterResponse},
        models::send_email_model::{EmailMessage, SendEmailModel},
    },
    try_get_surreal_pool,
    utils::infrastructure::functions::smtp::smtp_functions::SmtpFunctions,
};

#[async_trait::async_trait]
impl RegisterUseCaseTrait for RegisterUseCase {
    async fn register_user<'a>(
        &self,
        mut user_dto: RegisterDto,
        tx: &'a SendEmailModel,
    ) -> Result<JsonAdvanced<RegisterResponse>, UserConfigError> {
        println!("Start handler: {:?}", Utc::now());
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
        println!("After query: {:?}", Utc::now());
        let hash = Self::validate_password(&user_dto.hash)?;
        let hash =
            PasswordFunctions::hash_password(hash.as_str()).map_err(|_| UserConfigError {
                message: format!("Failed to hash password"),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;

        user_dto.hash = hash;

        // Validar usuario primero (porque create_user depende de esto)
        println!("Validating user: {:?}", user_dto);
        println!("Before query: {:?}", Utc::now());
        let user = self
            .validate_user(&mut user_dto, &conn.as_ref().client)
            .await?;
        println!("After query: {:?}", Utc::now());
        println!("User validated: {:?}", user);
        // Generar código y crear usuario paralelamente
        let (id, verify_code) = tokio::join!(
            self.create_user(&user_dto, &user, &conn.as_ref().client),
            self.generate_verify_code()
        );
        println!("Generated verify code: {:?}", verify_code);

        let id = id?;
        let verify_code = verify_code?;

        // Guardar código y enviar email paralelamente
        self.save_verify_code(&id, &verify_code, &conn.as_ref().client)
            .await?;
        tx.tx
            .send(EmailMessage {
                code: verify_code.clone(),
                email: user_dto.email.clone(),
                db: conn.as_ref().client.clone(),
            })
            .await
            .map_err(|_| UserConfigError {
                message: format!("Failed to send email"),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        println!("Returning response: {:?}", Utc::now());

        Ok(JsonAdvanced(RegisterResponse { verify_code }))
    }
}
#[async_trait::async_trait]
impl RegisterUseCasePrivate for RegisterUseCase {
    fn validate_password(password: &str) -> Result<String, UserConfigError> {
        if password.len() < 8 {
            return Err(UserConfigError {
                message: "Password must be at least 8 characters long".to_string(),
                status_code: StatusCode::BAD_REQUEST,
            });
        }
        let mut has_uppercase = false;
        let mut has_lowercase = false;
        let mut has_digit = false;
        let mut has_special = false;
        for c in password.chars() {
            if c.is_uppercase() {
                has_uppercase = true;
            }
            if c.is_lowercase() {
                has_lowercase = true;
            }
            if c.is_digit(10) {
                has_digit = true;
            }
            if "!@#$%^&*()_+-=[]{}|;':\",.<>?/".contains(c) {
                has_special = true;
            }
        }
        if !has_uppercase {
            return Err(UserConfigError {
                message: "Password must contain at least one uppercase letter".to_string(),
                status_code: StatusCode::BAD_REQUEST,
            });
        }
        if !has_lowercase {
            return Err(UserConfigError {
                message: "Password must contain at least one lowercase letter".to_string(),
                status_code: StatusCode::BAD_REQUEST,
            });
        }
        if !has_digit {
            return Err(UserConfigError {
                message: "Password must contain at least one number".to_string(),
                status_code: StatusCode::BAD_REQUEST,
            });
        }
        if !has_special {
            return Err(UserConfigError {
                message: "Password must contain at least one special character".to_string(),
                status_code: StatusCode::BAD_REQUEST,
            });
        }

        Ok(password.to_string())
    }
    async fn validate_user<'a>(
        &self,
        user_dto: &mut RegisterDto,
        db: &'a Surreal<Any>,
    ) -> Result<Option<UserConfigId>, UserConfigError> {
        if user_dto.email.is_empty() {
            return Err(UserConfigError {
                message: "Email cannot be empty".to_string(),
                status_code: StatusCode::BAD_REQUEST,
            });
        }
        if !user_dto.email.contains('@') || !checkmail::validate_email(&user_dto.email) {
            return Err(UserConfigError {
                message: "Invalid email format".to_string(),
                status_code: StatusCode::BAD_REQUEST,
            });
        }
        if user_dto.name.is_empty() {
            return Err(UserConfigError {
                message: "Name cannot be empty".to_string(),
                status_code: StatusCode::BAD_REQUEST,
            });
        }
        if user_dto.surnames.is_empty() {
            return Err(UserConfigError {
                message: "Surnames cannot be empty".to_string(),
                status_code: StatusCode::BAD_REQUEST,
            });
        }

        // Ahora consulta DB solo si todo válido:
        println!("[{}] Starting DB query: validate_user", Utc::now());
        let value: OneOrMany<UserConfigId> = execute_select_query(
            Query::<UserConfig>::new()
                .from(None, true)
                .with_index(&["idx_email"])
                .fields(&["id", "is_verified"])
                .condition(comparison("email", Operator::Eq, "email"))
                .condition(comparison("deleted_at", Operator::Eq, "$del"))
                .parameter("del", Value::from(None::<String>))
                .parameter("email", Value::from(user_dto.email.as_str()))
                .get_owned(),
            &db,
            false,
        )
        .await?;
        println!("[{}] Finished DB query: validate_user", Utc::now());

        // Maneja resultado
        if let OneOrMany::One(Some(user_id)) = value {
            if user_id.is_verified {
                return Err(UserConfigError {
                    message: "User already registered and verified".to_string(),
                    status_code: StatusCode::CONFLICT,
                });
            }
            return Ok(Some(user_id));
        }

        Ok(None)
    }

    async fn create_user<'a>(
        &self,
        user_dto: &RegisterDto,
        user_id: &Option<UserConfigId>,
        db: &'a Surreal<Any>,
    ) -> Result<RecordId, UserConfigError> {
        let user_config = UserConfig {
            id: None,
            email: user_dto.email.clone(),
            name: user_dto.name.clone(),
            surnames: user_dto.surnames.clone(),
            user_type: user_dto.user_type.clone(),
            hash: user_dto.hash.clone(),
            sessions: vec![],
            timestamp: Default::default(), // Asumiendo que Timestamp tiene un valor por defecto
            is_verified: false,
            status: StatusEnum::Active,
        };
        if !user_id.is_some() {
            println!("[{}] Starting DB operation: create user", Utc::now());
            let user_created = db
                .create::<Option<UserConfig>>(UserConfig::table_name())
                .content(user_config)
                .await
                .map_err(|e| UserConfigError {
                    message: format!("Failed to create user: {}", e),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })?;
            println!("[{}] Finished DB operation: create user", Utc::now());
            if user_created.is_none() {
                return Err(UserConfigError {
                    message: "Failed to create user".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                });
            }
            use ac_struct_back::schemas::auth::user_email_verify::user_email_verify::UserEmailVerify;
            use std::default::Default;
            // Aquí podrías guardar el usuario creado en una variable o hacer algo más con él
            let id = user_created.unwrap().id.unwrap().clone();

            let user_email_verify = UserEmailVerify {
                user_id: id.clone(),
                ..Default::default()
            };
            //crealo
            println!(
                "[{}] Starting DB operation: create user email verify",
                Utc::now()
            );
            let user_email = db
                .create::<Option<UserEmailVerify>>(UserEmailVerify::table_name())
                .content(user_email_verify)
                .await
                .map_err(|e| UserConfigError {
                    message: format!("Failed to create user email verify: {}", e),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })?;
            println!(
                "[{}] Finished DB operation: create user email verify",
                Utc::now()
            );
            // Si todo sale bien, retornamos Ok
            if user_email.is_none() {
                return Err(UserConfigError {
                    message: "Failed to create user email verify".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                });
            }

            Ok(id.clone())
        } else {
            let user_id_ref = user_id
                .as_ref()
                .and_then(|u| u.id.as_ref())
                .ok_or_else(|| UserConfigError {
                    message: "Invalid user ID".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })?;
            use smallvec::smallvec;
            println!("[{}] Starting DB operation: update user", Utc::now());
            let update_response: Vec<UserConfigId> = execute_update_query(
                UpdateRequest::<UserConfig>::builder()
                    .update(Some(UpdateTarget::Subquery(
                        Query::<UserConfig>::new()
                            .from(Some(user_id_ref.key().clone().to_string().as_str()), false)
                            .get_owned(),
                    )))
                    .map_err(|e| UserConfigError {
                        message: format!("Failed to build update query: {}", e),
                        status_code: StatusCode::INTERNAL_SERVER_ERROR,
                    })? //user_id_ref.clone().key().to_string().as_str()))
                    .set("hash", "h")
                    .set("name", "n")
                    .set("surnames", "s")
                    .parameter("h", Value::from(user_dto.hash.as_str()))
                    .parameter("n", Value::from(user_dto.name.as_str()))
                    .parameter("s", Value::from(user_dto.surnames.as_str()))
                    .return_clause(ReturnClause::Fields(smallvec![
                        Cow::Borrowed("id"),
                        Cow::Borrowed("is_verified"),
                    ]))
                    .get_owned(),
                db,
                false,
            )
            .await?;
            println!("[{}] Finished DB operation: update user", Utc::now());
            if update_response.len() != 1 {
                return Err(UserConfigError {
                    message: "Failed to update user hash".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                });
            }

            Ok(user_id_ref.clone())
        }
    }
    async fn generate_verify_code(&self) -> Result<String, UserConfigError> {
        let code = RandomCodeGenerator::generate_unique_code().map_err(|e| UserConfigError {
            message: format!("Failed to generate verification code: {}", e),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })?;
        Ok(format!("{:06}", code)) // Formatear el código a 6 dígitos
    }
    async fn save_verify_code<'a>(
        &self,
        user_dto: &RecordId,
        verify_code: &str,
        db: &'a Surreal<Any>,
    ) -> Result<(), UserConfigError> {
        println!("[{}] Starting DB operation: save verify code", Utc::now());
        let update_response: Vec<UserEmailVerifyId> = execute_update_query(
            UpdateRequest::<UserEmailVerify>::builder()
                .update(Some(UpdateTarget::Subquery(
                    Query::<UserEmailVerify>::new()
                        .from(None, false)
                        .condition(Condition::RecordIdComparission {
                            field: "user_id".into(),
                            op: Operator::Eq,
                            value: "$user".into(),
                        })
                        .fields(&["id"])
                        .get_owned(),
                )))
                .map_err(|e| UserConfigError {
                    message: format!("Failed to build update query: {}", e),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })? //user_dto.id.as_ref().unwrap().key().to_string().as_str()))
                .set("verify_code", "v")
                .set("last_verify_code_at", "l")
                .parameter("user", Value::from(user_dto.to_string().as_str()))
                .parameter("v", Value::from(verify_code))
                .parameter("l", serde_json::to_value(Utc::now()).unwrap())
                .return_clause(ReturnClause::Fields(smallvec![Cow::Borrowed("id")]))
                .get_owned(),
            db,
            false,
        )
        .await?;
        println!("[{}] Finished DB operation: save verify code", Utc::now());
        if update_response.len() != 1 {
            return Err(UserConfigError {
                message: "Failed to save verification code".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }

        Ok(())
    }
    // Ahora el método recibe Arc<Self>, email y verify_code como Arc<str>
    async fn send_email<'a>(
        email: String,
        verify_code: String,
        db: &'a Surreal<Any>,
    ) -> Result<(), UserConfigError> {
        // Aquí usas email y verify_code clonados con Arc (que es cheap clone)
        //get new email_template
        let email_type: OneOrMany<TemplateTypeId> = execute_select_query(
            Query::<TemplateType>::new()
                .from(None, true)
                .condition(comparison("name", Operator::Eq, "type"))
                .fields(&["id"])
                .parameter("type", Value::from("RE"))
                .get_owned(),
            db,
            true,
        )
        .await?;
        println!("passo 1");
        let template_type_id = match email_type {
            OneOrMany::One(email_type) => {
                if email_type.is_none() {
                    return Err(UserConfigError {
                        message: "Error al crear el tipo de plantilla".to_string(),
                        status_code: StatusCode::INTERNAL_SERVER_ERROR,
                    });
                }
                email_type.unwrap().id.unwrap()
            }
            OneOrMany::Many(_) => {
                return Err(UserConfigError {
                    message: "Error al crear el tipo de plantilla".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                });
            }
        };
        println!("passo 2");
        let email_template: OneOrMany<Template> = execute_select_query(
            Query::<Template>::new()
                .from(None, true)
                .condition(record_id_comparison("type_id", Operator::Eq, "$type_id"))
                .parameter(
                    "type_id",
                    serde_json::to_value(template_type_id.to_string()).unwrap(),
                )
                .get_owned(),
            db,
            true,
        )
        .await?;
        println!("passo 3");
        let (template_str, required_fields) = match email_template {
            OneOrMany::One(email_template) => {
                if email_template.is_none() {
                    return Err(UserConfigError {
                        message: "Error al crear el tipo de plantilla".to_string(),
                        status_code: StatusCode::INTERNAL_SERVER_ERROR,
                    });
                }
                let email_template = email_template.as_ref().unwrap();
                (
                    email_template.template_str.clone(),
                    email_template.required_fields.clone(),
                )
            }
            OneOrMany::Many(_) => {
                return Err(UserConfigError {
                    message: "Error al crear el tipo de plantilla".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                });
            }
        };
        //reemplazar todos los required fields que estan dentro del string con {@required_field}
        //whatsapp
        fn build_fields_map(verify_code: &[char]) -> HashMap<String, String> {
            let mut fields_values = HashMap::new();

            // Mapear D1-D6 con los caracteres del código
            for (i, d) in verify_code.iter().enumerate().take(6) {
                let key = format!("D{}", i + 1);
                fields_values.insert(key, d.to_string());
            }

            // Valores por defecto para otros campos
            fields_values.insert(
                "WHATSAPP_URL".to_string(),
                "https://wa.me/1234567890".to_string(),
            );
            fields_values.insert(
                "INSTAGRAM_URL".to_string(),
                "https://instagram.com/vessel".to_string(),
            );
            fields_values.insert(
                "TIKTOK_URL".to_string(),
                "https://tiktok.com/@vessel".to_string(),
            );
            fields_values.insert(
                "YOUTUBE_URL".to_string(),
                "https://youtube.com/vessel".to_string(),
            );
            fields_values.insert(
                "VERIFICATION_URL".to_string(),
                "https://vessel.com/verify".to_string(),
            );

            // Año actual
            let year = Utc::now().year();
            fields_values.insert("YEAR".to_string(), year.to_string());

            fields_values
        }

        let fields_map = build_fields_map(&verify_code.chars().collect::<Vec<char>>());
        //verificar que las claves del mapa sean las mismas que la variable required_fields con ==
        //y que los valores sean correctos
        println!("Fields map: {:?}", fields_map);
        for required_field in required_fields {
            println!("required_field: {:?}", required_field);
            if !fields_map.contains_key(&required_field) {
                println!("Error al crear el tipo de plantilla fallo en el required_field");
                return Err(UserConfigError {
                    message: "Error al crear el tipo de plantilla".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                });
            }
            let value = fields_map.get(&required_field).unwrap();
            if value.is_empty() {
                println!("Error al crear el tipo de plantilla fallo en el value");
                return Err(UserConfigError {
                    message: "Error al crear el tipo de plantilla".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                });
            }
        }
        fn replace_fields(template_str: &str, fields_values: &HashMap<String, String>) -> String {
            let mut result = template_str.to_owned();
            for (field, value) in fields_values {
                let placeholder = format!("{{{}}}", field); // ojo, en tu HTML usas {D1} no {@D1}
                result = result.replace(&placeholder, value);
            }
            result
        }
        let template_str = replace_fields(&template_str, &fields_map);
        println!("llego al envio");
        let email_send = SmtpFunctions::send_email(&email, "Registro de Usuario", &template_str)
            .await
            .map_err(|e| UserConfigError {
                message: format!("Failed to send email: {}", e),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        println!("Email sent: {:?}", email_send);

        Ok(())
    }
}
