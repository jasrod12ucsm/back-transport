use ac_struct_back::{
    schemas::config::template::template::{GetTemplatesError, Template},
    utils::domain::front_query::QueryFront,
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

pub struct GetOneTemplateUseCase;
#[async_trait::async_trait]
pub trait GetOneTemplateUseCasePublic {
    async fn execute(
        query: QueryFront<Template>,
        id: &str,
    ) -> Result<JsonAdvanced<Option<Template>>, GetTemplatesError>;
}
