use std::sync::Arc;

use ac_struct_back::{
    import::macro_import::async_trait,
    schemas::auth::user::user::{
        UserConfigError, registeruserdtouserconfig::RegisterUserDto,
        userconfigiduserconfig::UserConfigId,
    },
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::web::types::State;
use surrealdb::{RecordId, Surreal, engine::any::Any};

use crate::modules::user_management::domain::{
    data::{register_dto::RegisterDto, register_response::RegisterResponse},
    models::send_email_model::SendEmailModel,
};

pub struct RegisterUseCase;

impl RegisterUseCase {
    pub fn new() -> Self {
        RegisterUseCase {}
    }
}
#[async_trait::async_trait]
pub trait RegisterUseCaseTrait {
    async fn register_user<'a>(
        &self,
        user_dto: RegisterDto,
        tx: &'a SendEmailModel,
    ) -> Result<JsonAdvanced<RegisterResponse>, UserConfigError>;
}

#[async_trait::async_trait]
pub trait RegisterUseCasePrivate {
    fn validate_password(password: &str) -> Result<String, UserConfigError>;
    async fn validate_user<'a>(
        &self,
        user_dto: &mut RegisterDto,
        db: &'a Surreal<Any>,
    ) -> Result<Option<UserConfigId>, UserConfigError>;
    async fn create_user<'a>(
        &self,
        user_dto: &RegisterDto,
        user_id: &Option<UserConfigId>,
        db: &'a Surreal<Any>,
    ) -> Result<RecordId, UserConfigError>;
    async fn generate_verify_code(&self) -> Result<String, UserConfigError>;
    async fn save_verify_code<'a>(
        &self,
        user_dto: &RecordId,
        verify_code: &str,
        db: &'a Surreal<Any>,
    ) -> Result<(), UserConfigError>;
    async fn send_email<'a>(
        email: String,
        verify_code: String,
        db: &'a Surreal<Any>,
    ) -> Result<(), UserConfigError>;
}
