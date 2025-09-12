use common::utils::ntex_private::extractors::json::JsonAdvanced;

use crate::{
    modules::app_charge::domain::models::proyect_desnormalized::ProyectDesnormalized,
    utils::errors::csv_error::CsvError,
};
pub struct GetAllDataUseCase;

#[async_trait::async_trait]
pub trait GetAllDataUseCaseTrait {
    async fn execute(id: String) -> Result<JsonAdvanced<ProyectDesnormalized>, CsvError>;
}
