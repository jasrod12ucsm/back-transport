use common::utils::ntex_private::extractors::{
    json::JsonAdvanced, multipart_extractor::MultipartData,
};

use crate::{
    modules::app_charge::domain::response::file_charge_response::FileChargeResponse,
    utils::{charge_models::void_struct::VoidStruct, errors::csv_error::CsvError},
};

pub struct CHargeCategoricalUseCase;
#[async_trait::async_trait]
pub trait ChargeCategoricalUseCaseTrait {
    async fn charge_categorical_use_case(
        &self,
        data: MultipartData<VoidStruct>,
        proyect_id: String,
    ) -> Result<JsonAdvanced<FileChargeResponse>, CsvError>;
}
