use ac_struct_back::schemas::config::
    template::template::{
        createtemplatedtotemplate::CreateTemplateDto, CreateTemplateError, Template,
    }
;
use common::utils::ntex_private::extractors::json::JsonAdvanced;

pub struct CreateTemplateUseCase;
#[async_trait::async_trait]
pub trait CreateTemplateUseCasePublic {
    async fn execute(
        &self,
        dto: CreateTemplateDto,
    ) -> Result<JsonAdvanced<Template>, CreateTemplateError>;
}
