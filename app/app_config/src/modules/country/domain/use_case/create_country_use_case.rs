use ac_struct_back::schemas::config::country::country::{Country, UpdateCountryError};
use common::utils::ntex_private::extractors::{
    json::JsonAdvanced, multipart_extractor::MultipartData,
};

use crate::{
    modules::country::domain::data::create_country_response::CreateCountryResponse,
    utils::charge_models::void_struct::VoidStruct,
};
pub struct CreateCountryUseCase;
#[async_trait::async_trait]
pub trait CreateCountryUseCaseTrait {
    async fn create_country(
        dto: MultipartData<VoidStruct>,
    ) -> Result<JsonAdvanced<CreateCountryResponse>, UpdateCountryError>;
}
