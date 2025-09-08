use ac_struct_back::schemas::config::timezone::timezone::{CreateTimezoneError, Timezone};
use common::utils::ntex_private::extractors::{
    json::JsonAdvanced, multipart_extractor::MultipartData,
};
use ntex::web;

use crate::{
    modules::timezone::domain::use_case::create_timezone_use_case::{
        CreateTimezoneUseCase, CreateTimezoneUseCaseTrait,
    },
    utils::charge_models::void_struct::VoidStruct,
};

#[web::post("/")]
pub async fn create_timezone(
    dto: MultipartData<VoidStruct>,
) -> Result<JsonAdvanced<Vec<Timezone>>, CreateTimezoneError> {
    CreateTimezoneUseCase::execute(dto).await.map(JsonAdvanced)
}
