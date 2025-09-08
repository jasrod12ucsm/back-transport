use ac_struct_back::{
    schemas::config::template_type::template_type::{
        TemplateType, TemplateTypeError, UpdateTemplateTypeError,
        createtemplatetypedtotemplatetype::CreateTemplateTypeDto,
        updatetemplatetypedtotemplatetype::UpdateTemplateTypeDto,
    },
    utils::domain::{front_query::UpdateRequestBuilderFront, query::UpdateRequest},
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

pub struct UpdateTemplateTypeUseCase;
#[async_trait::async_trait]
pub trait UpdateTemplateTypeUseCasePublic {
    async fn execute(
        id: Option<String>,
        request: UpdateRequestBuilderFront<TemplateType>,
        dto: UpdateTemplateTypeDto,
    ) -> Result<JsonAdvanced<Vec<TemplateType>>, UpdateTemplateTypeError>;
}
