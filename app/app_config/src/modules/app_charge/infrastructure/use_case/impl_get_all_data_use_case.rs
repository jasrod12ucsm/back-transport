use std::{borrow::Cow, cell::RefCell, collections::HashMap, sync::RwLock};

use ac_struct_back::{
    import::macro_import::TableName,
    schemas::config::{
        feature::feature::Feature, proyect::proyect::Proyect,
        proyect_feature::proyect_feature::ProyectFeature,
    },
    utils::domain::query::{GraphBuilder, OneOrMany, Query, execute_select_query},
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use once_cell::sync::Lazy;
use polars::frame::DataFrame;

use crate::{
    modules::app_charge::domain::{
        models::proyect_desnormalized::ProyectDesnormalized,
        use_case::{
            charge_file_use_case::ChargeFileUseCase,
            get_all_data_use_case::{GetAllDataUseCase, GetAllDataUseCaseTrait},
        },
    },
    try_get_surreal_pool,
    utils::errors::csv_error::CsvError,
};
pub static DATA_FRAMES: Lazy<RwLock<HashMap<String, DataFrame>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

#[async_trait::async_trait]
impl GetAllDataUseCaseTrait for GetAllDataUseCase {
    async fn execute(id: String) -> Result<JsonAdvanced<ProyectDesnormalized>, CsvError> {
        let db = &try_get_surreal_pool()
            .ok_or_else(|| CsvError::FileChargeError)?
            .get()
            .await
            .map_err(|e| {
                println!("{:?}", e);
                CsvError::FileChargeError
            })?
            .client;
        let graph_expression = GraphBuilder::new()
            .out(ProyectFeature::table_name())
            .out(Feature::table_name())
            .project_object()
            .build()
            .build();
        let exist: OneOrMany<ProyectDesnormalized> = execute_select_query(
            Query::<Proyect>::new()
                .from(Some(&id), false)
                .add_field(
                    ac_struct_back::utils::domain::query::Expression::Graph(Box::new(
                        graph_expression,
                    )),
                    Some("fields"),
                )
                .add_field(
                    ac_struct_back::utils::domain::query::Expression::Field(Cow::Borrowed("id")),
                    None,
                )
                .get_owned(),
            db,
            false,
        )
        .await?;

        match exist {
            OneOrMany::One(_) => Err(CsvError::FileChargeError),
            OneOrMany::Many(val) => {
                if val.len() > 1 || val.is_empty() {
                    Err(CsvError::FileChargeError)
                } else {
                    Ok(JsonAdvanced(val.into_iter().next().unwrap()))
                }
            }
        }
    }
}
