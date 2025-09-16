use crate::{
    modules::app_charge::domain::response::categorical_to_categorical_response::CategoricalPlotResponse,
    utils::errors::csv_error::CsvError,
};

pub struct GetCategoryPlotUseCase {}
#[async_trait::async_trait]
pub trait GetCategoryPlotUseCaseTrait {
    async fn execute(&self, proyect_id: String) -> Result<CategoricalPlotResponse, CsvError>;
}
