use ac_struct_back::{
    schemas::config::country::country::{Country, UpdateCountryError},
    utils::domain::front_query::UpdateRequestBuilderFront,
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

use crate::modules::country::domain::data::update_country_dto::UpdateCountryDto;

pub struct UpdateCountryUseCase;
#[async_trait::async_trait]
pub trait UpdateCountryUseCaseTrait {
    async fn execute(
        id: Option<String>,
        request: UpdateRequestBuilderFront<Country>,
        dto: UpdateCountryDto,
    ) -> Result<JsonAdvanced<Vec<Country>>, UpdateCountryError>;
}
