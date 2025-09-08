use crate::modules::country::domain::{
    data::{create_country_response::CreateCountryResponse, update_country_dto::UpdateCountryDto},
    use_case::{
        create_country_use_case::{CreateCountryUseCase, CreateCountryUseCaseTrait},
        get_country_use_case::{GetCountryUseCase, GetCountryUseCaseTrait},
        get_one_country_use_case::{GetOneCountryUseCase, GetOneCountryeUseCaseTrait},
        update_country_use_case::{UpdateCountryUseCase, UpdateCountryUseCaseTrait},
    },
};
use ac_struct_back::{
    schemas::config::country::country::{
        Country, GetCountryError, GetOneCountryError, UpdateCountryError,
    },
    utils::domain::front_query::{QueryFront, UpdateRequestBuilderFront},
};
use common::utils::ntex_private::extractors::{
    json::JsonAdvanced, multipart_extractor::MultipartData, query_advanced::QueryAdvanced,
};
use ntex::{
    http::StatusCode,
    web::{self, types::Path},
};

use crate::utils::charge_models::void_struct::VoidStruct;

#[web::post("/")]
async fn create_countries_by_doc(
    dto: MultipartData<VoidStruct>,
) -> Result<JsonAdvanced<CreateCountryResponse>, UpdateCountryError> {
    CreateCountryUseCase::create_country(dto).await
}

#[web::get("/")]
async fn get_countries(
    query: QueryAdvanced<QueryFront<Country>>,
) -> Result<JsonAdvanced<Vec<Country>>, GetCountryError> {
    GetCountryUseCase::execute(query.0).await
}
//get_one
#[web::get("/{id}")]
async fn get_one_country(
    path: Path<String>,
    query: QueryAdvanced<QueryFront<Country>>,
) -> Result<JsonAdvanced<Option<Country>>, GetOneCountryError> {
    let id = path.into_inner();
    GetOneCountryUseCase::execute(query.0, id.as_str()).await
}

#[web::patch("/")]
async fn update_country(
    dto: JsonAdvanced<UpdateCountryDto>,
    query: QueryAdvanced<UpdateRequestBuilderFront<Country>>,
) -> Result<JsonAdvanced<Vec<Country>>, UpdateCountryError> {
    UpdateCountryUseCase::execute(None, query.0, dto.0).await
}

#[web::patch("/{id}")]
async fn update_country_by_id(
    path: Path<String>,
    dto: JsonAdvanced<UpdateCountryDto>,
    query: QueryAdvanced<UpdateRequestBuilderFront<Country>>,
) -> Result<JsonAdvanced<Country>, UpdateCountryError> {
    let id = path.into_inner();
    UpdateCountryUseCase::execute(Some(id), query.0, dto.0)
        .await
        .and_then(|wrapped_vec| {
            // Suponiendo que wrapped_vec es JsonAdvanced<Vec<Country>>
            // y queremos obtener el primer Country, si existe
            match wrapped_vec.into_inner().into_iter().next() {
                Some(country) => Ok(JsonAdvanced(country)),
                None => Err(UpdateCountryError {
                    message: "No se encontró ningún Country".to_string(),
                    status_code: StatusCode::NOT_FOUND,
                }),
            }
        })
}
