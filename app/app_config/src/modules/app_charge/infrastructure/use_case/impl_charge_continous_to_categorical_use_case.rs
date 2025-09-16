use std::{ffi::CString, sync::Arc};

use ac_struct_back::{
    import::macro_import::TableName,
    schemas::config::{
        continous_to_categorical::{ContinousContent, ContinousToCategorical},
        feature::{feature::Feature, feature_type::FeatureType},
        proyect_feature::proyect_feature::ProyectFeature,
    },
    utils::domain::{query::GraphBuilder, relations::RelateRequest},
};
use ahash::HashSet;
use common::utils::ntex_private::extractors::{
    json::JsonAdvanced, multipart_extractor::MultipartData,
};
use pyo3::{
    Py, PyAny, PyResult, Python,
    types::{PyBytes, PyList, PyModule, PyString},
};
use serde_json::json;
use surrealdb::{Surreal, engine::any::Any};
use tokio::{sync::mpsc, task::JoinSet};

use crate::{
    PYTHON_ARCHIVE,
    modules::app_charge::domain::{
        models::features_for_proyect::FeaturesForProyect,
        response::file_charge_response::FileChargeResponse,
        use_case::charge_file_use_case::ChargeFileUseCase,
    },
    try_get_surreal_pool,
    utils::{charge_models::void_struct::VoidStruct, errors::csv_error::CsvError},
};

#[async_trait::async_trait]
pub trait ChargeContinuousCategoricalUseCaseTrait {
    async fn charge_continuous_categorical_use_case(
        &self,
        data: MultipartData<VoidStruct>,
        proyect_id: String,
    ) -> Result<JsonAdvanced<FileChargeResponse>, CsvError>;
}

pub struct ChargeContinuousCategoricalUseCase;

#[async_trait::async_trait]
impl ChargeContinuousCategoricalUseCaseTrait for ChargeContinuousCategoricalUseCase {
    async fn charge_continuous_categorical_use_case(
        &self,
        data: MultipartData<VoidStruct>,
        proyect_id: String,
    ) -> Result<JsonAdvanced<FileChargeResponse>, CsvError> {
        let db = try_get_surreal_pool()
            .ok_or_else(|| CsvError::FileChargeError)?
            .get()
            .await
            .map_err(|e| {
                println!("{:?}", e);
                CsvError::FileChargeError
            })?;
        let conn = db.client.clone();
        let (data_df, features) = Self::process_files(data, proyect_id.clone(), &conn).await?;
        Self::process_continuous_categorical(features, data_df, conn).await?;
        Ok(JsonAdvanced(FileChargeResponse { ok: true }))
    }
}

impl ChargeContinuousCategoricalUseCase {
    pub async fn process_pair_columns(
        category_col: FeaturesForProyect,
        continuous_col: FeaturesForProyect,
        dataframe: Arc<Py<PyAny>>,
    ) -> Result<RelateRequest<ContinousToCategorical>, CsvError> {
        let python_process: String = Python::with_gil(|py| -> PyResult<String> {
            let module = PyModule::from_code(
                py,
                CString::new(PYTHON_ARCHIVE).unwrap().as_c_str(),
                CString::new("main.py").unwrap().as_c_str(),
                CString::new("embedded").unwrap().as_c_str(),
            )?
            .unbind();

            let func = module.getattr(py, "category_boxplot_with_outliers")?;

            let df = func
                .call1(
                    py,
                    (
                        dataframe.as_ref(),
                        PyString::new(py, &category_col.name),
                        PyString::new(py, &continuous_col.name),
                    ),
                )?
                .into_any()
                .extract(py)?;

            Ok(df)
        })
        .map_err(|e| {
            eprintln!("Python error: {:?}", e);
            CsvError::FileChargeError
        })?;

        let result: ContinousContent = serde_json::from_str(&python_process).map_err(|e| {
            eprintln!("Python error: {:?}", e);
            CsvError::FileChargeError
        })?;
        let relate_request = RelateRequest::<ContinousToCategorical>::builder()
            .from(&category_col.id.key().to_string())
            .to(&continuous_col.id.key().to_string())
            .content(ContinousToCategorical {
                content: result,
                ..Default::default()
            })
            .map_err(|e| {
                println!("{:?}", e);
                CsvError::FileChargeError
            })?
            .get_owned()
            .build()
            .map_err(|e| {
                println!("{:?}", e);
                CsvError::FileChargeError
            })?;
        Ok(relate_request)
    }

    pub async fn process_continuous_categorical(
        features: Vec<FeaturesForProyect>,
        data: Py<PyAny>,
        conn: Surreal<Any>,
    ) -> Result<(), CsvError> {
        println!("llego a procesar los datos boxplot");
        let categorical: Vec<FeaturesForProyect> = features
            .iter()
            .filter(|f| matches!(f.type_feature.clone().unwrap(), FeatureType::Categorical))
            .cloned()
            .collect();
        let continuous: Vec<FeaturesForProyect> = features
            .iter()
            .filter(|f| matches!(f.type_feature.clone().unwrap(), FeatureType::Continuous))
            .cloned()
            .collect();
        let num_cat = categorical.len();
        let num_cont = continuous.len();
        if num_cat == 0 || num_cont == 0 {
            return Ok(());
        }

        let total_cores = num_cpus::get();
        let db_workers = (total_cores / 2).max(2);
        let processing_workers = total_cores * 2;

        println!(
            "Optimizando con {} cores totales: {} para DB, {} para procesamiento",
            total_cores, db_workers, processing_workers
        );

        let arc_conn = Arc::new(conn);

        let (tx, mut rx) =
            mpsc::channel::<RelateRequest<ContinousToCategorical>>(processing_workers * 10);

        let receiver_handle = tokio::spawn({
            let arc_conn = arc_conn.clone();
            async move {
                let mut db_join_set = JoinSet::new();
                let db_semaphore = Arc::new(tokio::sync::Semaphore::new(db_workers));

                while let Some(mut relate_request) = rx.recv().await {
                    let arc_conn = arc_conn.clone();
                    let permit = db_semaphore.clone().acquire_owned().await.unwrap();

                    db_join_set.spawn(async move {
                        let _permit = permit;
                        let mut relate_request = relate_request.build_surreal_query(false).unwrap();
                        if let Err(e) = arc_conn.query(relate_request).await {
                            eprintln!("Error ejecutando query: {:?}", e);
                        }
                    });

                    if db_join_set.len() >= db_workers * 2 {
                        let _ = db_join_set.join_next().await;
                    }
                }

                while let Some(res) = db_join_set.join_next().await {
                    if let Err(e) = res {
                        eprintln!("Error en tarea de DB: {:?}", e);
                    }
                }
            }
        });

        let mut processing_join_set = JoinSet::new();
        let processing_semaphore = Arc::new(tokio::sync::Semaphore::new(processing_workers));
        let data = Arc::new(data);

        for i in 0..num_cat {
            for j in 0..num_cont {
                let tx = tx.clone();
                let f1 = categorical[i].clone();
                let f2 = continuous[j].clone();
                let data = data.clone();
                let permit = processing_semaphore.clone().acquire_owned().await.unwrap();

                processing_join_set.spawn(async move {
                    let _permit = permit;

                    match tokio::task::spawn_blocking(move || {
                        Self::process_pair_columns(f1, f2, data)
                    })
                    .await
                    .unwrap()
                    .await
                    {
                        Ok(relate_request) => {
                            if let Err(e) = tx.send(relate_request).await {
                                eprintln!("Error enviando al canal: {:?}", e);
                            }
                        }
                        Err(e) => eprintln!("Error en spawn_blocking: {:?}", e),
                    }
                });
            }
        }
        while let Some(res) = processing_join_set.join_next().await {
            if let Err(e) = res {
                eprintln!("Error en procesamiento: {:?}", e);
            }
        }

        println!("termino processing");

        drop(tx);
        receiver_handle.await.unwrap();

        println!("termino processing");
        Ok(())
    }

    pub async fn process_files(
        mut data: MultipartData<VoidStruct>,
        proyect_id: String,
        db: &Surreal<Any>,
    ) -> Result<(Py<PyAny>, Vec<FeaturesForProyect>), CsvError> {
        println!("data {:?}", data.get_data());
        let preload_file = data.take_files().ok_or_else(|| CsvError::FileChargeError)?;
        if preload_file.is_empty() || preload_file.len() > 1 {
            return Err(CsvError::FileChargeError);
        }
        if preload_file.get(0).unwrap().extension != "csv" {
            return Err(CsvError::InvalidFileType);
        }
        let mut files = preload_file;
        let file = files.remove(0);
        let bytes = file.file_data;
        let features = Self::get_features_for_proyect(proyect_id.clone(), &db).await?;
        let set_map = features
            .iter()
            .map(|x| x.name.clone())
            .collect::<HashSet<String>>();
        let df: Py<PyAny> = Python::attach(|py| -> PyResult<Py<PyAny>> {
            let module = PyModule::from_code(
                py,
                CString::new(PYTHON_ARCHIVE).unwrap().as_c_str(),
                CString::new("main.py").unwrap().as_c_str(),
                CString::new("embedded").unwrap().as_c_str(),
            )?
            .unbind();

            let func = module.getattr(py, "process_dataframe_from_csv_bytes")?;

            let py_csv_bytes = PyBytes::new(py, &bytes).unbind();

            let py_columns = PyList::new(py, set_map.clone().iter()).unwrap().unbind();

            let df = func.call1(py, (py_csv_bytes, py_columns))?;

            Ok(df)
        })
        .map_err(|e| {
            eprintln!("Python error: {:?}", e);
            CsvError::FileChargeError
        })?;

        Ok((df, features))
    }

    async fn get_features_for_proyect(
        proyect_id: String,
        db: &Surreal<Any>,
    ) -> Result<Vec<FeaturesForProyect>, CsvError> {
        let params = json!({ "project" : format!("mst_proyect:{}", proyect_id) });
        let query = "RETURN (SELECT id,->mst_proyect_feature->(SELECT * FROM mst_feature where type_feature IN ['Categorical', 'Continuous']).{name,id,type_feature} as features from <record>$project).features[0];";
        let mut db = db.query(query).bind(params).await.map_err(|e| {
            println!("{:?}", e);
            CsvError::FileChargeError
        })?;
        let features: Vec<FeaturesForProyect> = db.take(0).map_err(|e| {
            println!("{:?}", e);
            CsvError::FileChargeError
        })?;
        println!("features: {:?}", features);
        Ok(features)
    }
}
