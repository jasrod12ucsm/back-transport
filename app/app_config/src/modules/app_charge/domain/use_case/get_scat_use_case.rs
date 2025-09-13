use common::utils::ntex_private::extractors::json::JsonAdvanced;

use crate::{
    modules::app_charge::domain::response::scatter_plot_response::ScatterPlotResponse,
    utils::errors::csv_error::CsvError,
};

pub struct ScatterPlotUseCase;

#[async_trait::async_trait]
pub trait ScatterPlotUseCaseTrait {
    async fn execute(
        self,
        id: String,
        scatt: String,
    ) -> Result<JsonAdvanced<ScatterPlotResponse>, CsvError>;
}
