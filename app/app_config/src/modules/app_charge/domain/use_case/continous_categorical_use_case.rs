use common::utils::ntex_private::extractors::json::JsonAdvanced;

use crate::{
    modules::app_charge::domain::response::continous_categorical_response::ContinousCategoricalResponse,
    utils::errors::csv_error::CsvError,
};

pub struct GetContinuousCategoricalUseCase {}
#[async_trait::async_trait]
pub trait GetContinuousCategoricalUseCaseTrait {
    async fn execute(
        &self,
        proyect_id: String,
    ) -> Result<JsonAdvanced<ContinousCategoricalResponse>, CsvError>;
}
