use std::sync::Arc;

use ac_struct_back::schemas::config::feature::feature::Feature;
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use futures::{StreamExt, stream::FuturesUnordered};
use serde_json::json;
use tokio::sync::Semaphore;

use crate::{
    modules::app_charge::domain::{
        response::scatter_plot_response::ScatterPlotResponse,
        use_case::get_scat_use_case::{ScatterPlotUseCase, ScatterPlotUseCaseTrait},
    },
    try_get_surreal_pool,
    utils::errors::csv_error::CsvError,
};

#[async_trait::async_trait]
impl ScatterPlotUseCaseTrait for ScatterPlotUseCase {
    async fn execute(
        self,
        id: String,
        scatt: String,
    ) -> Result<JsonAdvanced<ScatterPlotResponse>, CsvError> {
        let db = try_get_surreal_pool()
            .ok_or_else(|| CsvError::FileChargeError)?
            .get()
            .await
            .map_err(|e| {
                println!("{:?}", e);
                CsvError::FileChargeError
            })?;
        let param = json!(
        {
            "project_id": format!("mst_proyect:{}", id),
            "feature_id": format!("mst_feature:{}",scatt)
        }
        );
        let conn = &db.client;
        let mut features = conn
            .query(
                "SELECT * 
FROM <record>$feature_id
WHERE <-mst_proyect_feature<-mst_proyect CONTAINS <record>$project_id;",
            )
            .bind(param.clone())
            .await
            .map_err(|e| {
                println!("{:?}", e);
                CsvError::FileChargeError
            })?;
        let features: Vec<Feature> = features.take(0).map_err(|e| {
            println!("{:?}", e);
            CsvError::FileChargeError
        })?;
        let semaphore = Arc::new(Semaphore::new(10));
        let mut tasks = FuturesUnordered::new();
        for feature in features {
            let semaphore = semaphore.clone().acquire_owned().await.unwrap();
            let conn = conn.clone();
            let param = json!(
             {
                 "project_id": format!("mst_proyect:{}", id),
                 "feature_id": format!("mst_feature:{}",feature.id.as_ref().unwrap().key().to_string())
             }
            );
            println!("param : {:?}", param);

            tasks.push(async move {
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
                content_scatter,
                ->mst_feature.name[0] AS name
            FROM mst_feature_to_feature
        ).{out, content_scatter, name} AS scatter
    FROM <record>$feature_id
    WHERE ->mst_feature_to_feature
) AS features
FROM ONLY <record>$project_id
WHERE ->mst_proyect_feature->mst_feature->mst_feature_to_feature;
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
                value.take::<Option<ScatterPlotResponse>>(0).map_err(|e| {
                    println!("{:?}", e);
                    CsvError::FileChargeError
                })
            });
        }
        println!("aqui");
        let mut scatters = Vec::<ScatterPlotResponse>::new();
        while let Some(scatter_plot) = tasks.next().await {
            match scatter_plot {
                Ok(Some(scatter_plot)) => {
                    if scatters.len() == 0 {
                        scatters.push(scatter_plot);
                    } else {
                        if let Some(field) = scatter_plot.features.into_iter().next() {
                            scatters[0].features.push(field);
                        }
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    return Err(e);
                }
            }
        }
        Ok(JsonAdvanced(
            scatters
                .into_iter()
                .next()
                .ok_or(CsvError::FileChargeError)?,
        ))
    }
}
