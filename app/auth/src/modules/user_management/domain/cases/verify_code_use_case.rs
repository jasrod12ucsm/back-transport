use ac_struct_back::schemas::auth::user::user::UserConfigError;

pub struct VerifyCodeUseCase;

#[async_trait::async_trait]
pub trait VerifyCodeUseCaseTrait {
    async fn verify_code(&self, code: &str, email: &str) -> Result<String, UserConfigError>;
}
