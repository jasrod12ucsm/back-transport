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
                categorical_to_categorical_response::CategoricalPlotResponse,
                continous_categorical_response::ContinousCategoricalResponse,
                file_charge_response::FileChargeResponse, get_columns_response::GetColumnsResponse,
                scatter_plot_response::ScatterPlotResponse,
            },
            use_case::{
                charge_categorical_use_case::{
                    CHargeCategoricalUseCase, ChargeCategoricalUseCaseTrait,
                },
                charge_file_use_case::{ChargeFileUseCase, ChargeFileUseCaseTrait},
                continous_categorical_use_case::{
                    GetContinuousCategoricalUseCase, GetContinuousCategoricalUseCaseTrait,
                },
                get_all_data_use_case::{GetAllDataUseCase, GetAllDataUseCaseTrait},
                get_category_plot_use_case::{GetCategoryPlotUseCase, GetCategoryPlotUseCaseTrait},
                get_columns_use_case::{GetColumnsUseCase, GetColumnsUseCaseTrait},
                get_scat_use_case::{ScatterPlotUseCase, ScatterPlotUseCaseTrait},
            },
        },
        infrastructure::use_case::{
            impl_charge_continous_to_categorical_use_case::{
                ChargeContinuousCategoricalUseCase, ChargeContinuousCategoricalUseCaseTrait,
            },
            impl_charge_field_scatt_use_case::{
                ChargeFieldScatterUseCase, ChargeFieldScatterUseCaseTrait,
            },
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

#[web::post("charge_categoricals/{id}")]
async fn charge_categoricals(
    id: Path<String>,
    dto: MultipartData<VoidStruct>,
) -> Result<JsonAdvanced<FileChargeResponse>, CsvError> {
    CHargeCategoricalUseCase {}
        .charge_categorical_use_case(dto, id.into_inner())
        .await
}
//charge caontious categorical
#[web::post("/charge_continous/{id}")]
async fn charge_continous(
    id: Path<String>,
    dto: MultipartData<VoidStruct>,
) -> Result<JsonAdvanced<FileChargeResponse>, CsvError> {
    ChargeContinuousCategoricalUseCase {}
        .charge_continuous_categorical_use_case(dto, id.into_inner())
        .await
}

#[web::get("get_categorical_plot/{id}")]
async fn get_categorical_plot(
    id: Path<String>,
) -> Result<JsonAdvanced<CategoricalPlotResponse>, CsvError> {
    GetCategoryPlotUseCase {}
        .execute(id.into_inner())
        .await
        .map(|r| JsonAdvanced(r))
}

#[web::get("get_continous_categorical/{id}")]
async fn get_continous_categorical(
    id: Path<String>,
) -> Result<JsonAdvanced<ContinousCategoricalResponse>, CsvError> {
    GetContinuousCategoricalUseCase {}
        .execute(id.into_inner())
        .await
}
