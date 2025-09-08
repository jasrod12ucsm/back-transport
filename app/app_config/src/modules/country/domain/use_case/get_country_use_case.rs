use ac_struct_back::{
    schemas::config::country::country::{Country, GetCountryError},
    utils::domain::front_query::QueryFront,
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

pub struct GetCountryUseCase;
#[async_trait::async_trait]
pub trait GetCountryUseCaseTrait {
    async fn execute(
        query: QueryFront<Country>,
    ) -> Result<JsonAdvanced<Vec<Country>>, GetCountryError>;
}
