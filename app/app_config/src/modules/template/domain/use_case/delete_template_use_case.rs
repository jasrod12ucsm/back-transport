use ac_struct_back::schemas::config::template::template::{DeleteTemplateError, Template};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

pub struct DeleteTemplateUseCase;
#[async_trait::async_trait]
pub trait DeleteTemplateUseCasePublic {
    async fn execute(id: &str) -> Result<JsonAdvanced<Template>, DeleteTemplateError>;
}
