use common::utils::ntex_private::extractors::json::JsonAdvanced;
use serde_json::json;

use crate::{
    modules::app_charge::domain::{
        response::continous_categorical_response::ContinousCategoricalResponse,
        use_case::continous_categorical_use_case::{
            GetContinuousCategoricalUseCase, GetContinuousCategoricalUseCaseTrait,
        },
    },
    try_get_surreal_pool,
    utils::errors::csv_error::CsvError,
};

#[async_trait::async_trait]
impl GetContinuousCategoricalUseCaseTrait for GetContinuousCategoricalUseCase {
    async fn execute(
        &self,
        proyect_id: String,
    ) -> Result<JsonAdvanced<ContinousCategoricalResponse>, CsvError> {
        let db = try_get_surreal_pool()
            .ok_or_else(|| CsvError::FileChargeError)?
            .get()
            .await
            .map_err(|e| {
                println!("{:?}", e);
                CsvError::FileChargeError
            })?;
        let conn = db.client.clone();

        let param = json!(
        {
            "project_id": format!("mst_proyect:{}", proyect_id),
        }
        );
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
            FROM mst_continous_to_categorical
        ).{out, content, name} AS scatter
    FROM mst_feature
    WHERE ->mst_categorical_to_categorical
) AS features
FROM ONLY <record>$project_id
WHERE ->mst_proyect_feature->mst_feature->mst_continous_to_categorical;
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
            .take::<Option<ContinousCategoricalResponse>>(0)
            .map_err(|e| {
                println!("{:?}", e);
                CsvError::FileChargeError
            })?;
        if response.is_none() {
            return Err(CsvError::FileChargeError);
        }
        Ok(JsonAdvanced(response.unwrap()))
    }
}
