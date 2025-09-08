use ac_struct_back::schemas::config::template_type::template_type::{
    createtemplatetypedtotemplatetype::CreateTemplateTypeDto, TemplateType, TemplateTypeError,
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

pub struct CreateTemplateTypeUseCase;
#[async_trait::async_trait]
pub trait CreateTemplateTypeUseCasePublic {
    async fn execute(
        &self,
        template_type: CreateTemplateTypeDto,
    ) -> Result<JsonAdvanced<TemplateType>, TemplateTypeError>;
}
