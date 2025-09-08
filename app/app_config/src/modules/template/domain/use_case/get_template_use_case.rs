use ac_struct_back::{
    schemas::config::template::template::{GetTemplatesError, Template},
    utils::domain::front_query::QueryFront,
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

pub struct GetTemplatesUseCase;
#[async_trait::async_trait]
pub trait GetTemplatesUseCasePublic {
    async fn execute(
        query: QueryFront<Template>,
    ) -> Result<JsonAdvanced<Vec<Template>>, GetTemplatesError>;
}
