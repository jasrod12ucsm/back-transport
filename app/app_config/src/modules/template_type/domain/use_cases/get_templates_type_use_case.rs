use ac_struct_back::{
    schemas::config::template_type::template_type::{TemplateType, TemplateTypesNotFoundError},
    utils::domain::{front_query::QueryFront, query::Query},
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

pub struct GetTemplatesTypeUseCase;
#[async_trait::async_trait]
pub trait GetTemplatesTypeUseCasePublic {
    async fn execute(
        query: QueryFront<TemplateType>,
    ) -> Result<JsonAdvanced<Vec<TemplateType>>, TemplateTypesNotFoundError>;
}
