use ac_struct_back::{
    schemas::config::template::template::{
        Template, UpdateTemplateError, updatetemplatedtotemplate::UpdateTemplateDto,
    },
    utils::domain::front_query::UpdateRequestBuilderFront,
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

pub struct UpdateTemplateUseCase;
#[async_trait::async_trait]
pub trait UpdateTemplateUseCasePublic {
    async fn execute(
        id: Option<String>,
        request: UpdateRequestBuilderFront<Template>,
        dto: UpdateTemplateDto,
    ) -> Result<JsonAdvanced<Vec<Template>>, UpdateTemplateError>;
}
