use ac_struct_back::{
    schemas::config::country::country::{Country, GetOneCountryError},
    utils::domain::front_query::QueryFront,
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

pub struct GetOneCountryUseCase;
#[async_trait::async_trait]
pub trait GetOneCountryeUseCaseTrait {
    async fn execute(
        query: QueryFront<Country>,
        id: &str,
    ) -> Result<JsonAdvanced<Option<Country>>, GetOneCountryError>;
}
