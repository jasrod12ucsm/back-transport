use std::sync::Arc;

use common::utils::ntex_private::extractors::json::JsonAdvanced;
use serde_json::json;
use tokio::sync::Semaphore;

use crate::{
    modules::app_charge::domain::{
        response::categorical_to_categorical_response::CategoricalPlotResponse,
        use_case::get_category_plot_use_case::{
            GetCategoryPlotUseCase, GetCategoryPlotUseCaseTrait,
        },
    },
    try_get_surreal_pool,
    utils::errors::csv_error::CsvError,
};
#[async_trait::async_trait]
impl GetCategoryPlotUseCaseTrait for GetCategoryPlotUseCase {
    async fn execute(
        &self,
        id: String,
        num_page: String,
        real_num_page: i32,
    ) -> Result<CategoricalPlotResponse, CsvError> {
        let db = try_get_surreal_pool()
            .ok_or_else(|| CsvError::FileChargeError)?
            .get()
            .await
            .map_err(|e| {
                println!("{:?}", e);
                CsvError::FileChargeError
            })?;
        let start = (real_num_page - 1) * 3;
        let end = real_num_page * 3;

        let param = json!(
        {
            "project_id": format!("mst_proyect:{}", id),
            "feature_id": format!("mst_feature:{}",num_page),
            "static": start,
        }
        );
        let conn = &db.client;
        println!("param : {:?}", param);

        let mut value = conn
            .query(
                "
            
SELECT (
    SELECT
        id,
        name,
        (
            SELECT
                out,
                content,
                ->mst_feature.name[0] AS name
            FROM mst_categorical_to_categorical where in= $parent.id START AT $static LIMIT 3
        ).{out, content, name} AS scatter
    FROM <record>$feature_id
    WHERE ->mst_categorical_to_categorical
) AS features
FROM ONLY <record>$project_id
WHERE ->mst_proyect_feature->mst_feature->mst_categorical_to_categorical;
            ",
            )
            .bind(param)
            .await
            .map_err(|e| {
                println!("aqui el eerror");
                println!("{:?}", e);
                CsvError::FileChargeError
            })
            .unwrap();
        println!("valued : {:?}", value);
        let response = value
            .take::<Option<CategoricalPlotResponse>>(0)
            .map_err(|e| {
                println!("{:?}", e);
                CsvError::FileChargeError
            })?;
        if response.is_none() {
            return Err(CsvError::FileChargeError);
        }
        println!("aqui");
        Ok(response.unwrap())
    }
}
