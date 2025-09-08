



use ac_struct_back::schemas::config::template_type::template_type::{
    DeleteTemplateTypeError, TemplateType
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

pub struct DeleteTemplateTypeUseCase;
#[async_trait::async_trait]
pub trait DeleteTemplateTypeUseCasePublic {
    async fn execute(
        template_type: &str,
    ) -> Result<JsonAdvanced<TemplateType>, DeleteTemplateTypeError>;
}
