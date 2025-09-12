use common::utils::ntex_private::extractors::{
    json::JsonAdvanced, multipart_extractor::MultipartData,
};

use crate::{
    modules::app_charge::domain::response::get_columns_response::GetColumnsResponse,
    utils::{charge_models::void_struct::VoidStruct, errors::csv_error::CsvError},
};
pub struct GetColumnsUseCase;

#[async_trait::async_trait]
pub trait GetColumnsUseCaseTrait {
    async fn charge_file(
        dto: MultipartData<VoidStruct>,
    ) -> Result<JsonAdvanced<GetColumnsResponse>, CsvError>;
}
