use ac_struct_back::schemas::config::{
    country::country::{Country, UpdateCountryError},
    feature::feature::CreateFeatureError,
};
use common::utils::ntex_private::extractors::{
    json::JsonAdvanced, multipart_extractor::MultipartData,
};

use crate::{
    modules::app_charge::domain::{
        data::charge_dto::ChargeDto, response::file_charge_response::FileChargeResponse,
    },
    utils::{charge_models::void_struct::VoidStruct, errors::csv_error::CsvError},
};
pub struct ChargeFileUseCase;
#[async_trait::async_trait]
pub trait ChargeFileUseCaseTrait {
    async fn charge_file(
        dto: MultipartData<ChargeDto>,
        proyect_id: String,
    ) -> Result<JsonAdvanced<FileChargeResponse>, CsvError>;
}
