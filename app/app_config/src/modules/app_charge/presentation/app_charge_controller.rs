use ac_struct_back::schemas::config::feature::feature::CreateFeatureError;
use common::utils::ntex_private::extractors::{
    json::JsonAdvanced, multipart_extractor::MultipartData,
};
use ntex::web::{self, types::Path};

use crate::{
    modules::app_charge::{
        domain::{
            data::charge_dto::ChargeDto,
            models::proyect_desnormalized::ProyectDesnormalized,
            response::{
                file_charge_response::FileChargeResponse, get_columns_response::GetColumnsResponse,
                scatter_plot_response::ScatterPlotResponse,
            },
            use_case::{
                charge_file_use_case::{ChargeFileUseCase, ChargeFileUseCaseTrait},
                get_all_data_use_case::{GetAllDataUseCase, GetAllDataUseCaseTrait},
                get_columns_use_case::{GetColumnsUseCase, GetColumnsUseCaseTrait},
                get_scat_use_case::{ScatterPlotUseCase, ScatterPlotUseCaseTrait},
            },
        },
        infrastructure::use_case::impl_charge_field_scatt_use_case::{
            ChargeFieldScatterUseCase, ChargeFieldScatterUseCaseTrait,
        },
    },
    utils::{charge_models::void_struct::VoidStruct, errors::csv_error::CsvError},
};

#[web::post("/charge/{id}")]
async fn charge_file(
    dto: MultipartData<ChargeDto>,
    id: Path<String>,
) -> Result<JsonAdvanced<FileChargeResponse>, CsvError> {
    ChargeFileUseCase::charge_file(dto, id.into_inner()).await
}

#[web::post("/columns")]
async fn get_columns(
    dto: MultipartData<VoidStruct>,
) -> Result<JsonAdvanced<GetColumnsResponse>, CsvError> {
    GetColumnsUseCase::charge_file(dto).await
}
#[web::get("/all_data/{id}")]
async fn get_all_data(id: Path<String>) -> Result<JsonAdvanced<ProyectDesnormalized>, CsvError> {
    GetAllDataUseCase::execute(id.into_inner()).await
}

#[web::get("/get_scat/{id}/{scatt}")]
async fn get_scat(
    id: Path<(String, String)>,
) -> Result<JsonAdvanced<ScatterPlotResponse>, CsvError> {
    let id = id.into_inner();
    ScatterPlotUseCase {}.execute(id.0, id.1).await
}

#[web::post("/charge_field_scatt/{id}/{scatt}")]
async fn charge_field_scatt(
    id: Path<(String, String)>,
    dto: MultipartData<VoidStruct>,
) -> Result<JsonAdvanced<FileChargeResponse>, CsvError> {
    let proyect_id = id.into_inner();
    ChargeFieldScatterUseCase::charge_file(dto, proyect_id.0, proyect_id.1).await
}
