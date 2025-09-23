use std::{ffi::CString, sync::Arc};

use ac_struct_back::{
    import::macro_import::TableName,
    schemas::config::{
        categorical_to_categorical::{CategoricalContent, CategoricalToCategorical},
        feature::feature::Feature,
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
        use_case::{
            charge_categorical_use_case::{
                CHargeCategoricalUseCase, ChargeCategoricalUseCaseTrait,
            },
            charge_file_use_case::ChargeFileUseCase,
        },
    },
    try_get_surreal_pool,
    utils::{charge_models::void_struct::VoidStruct, errors::csv_error::CsvError},
};
#[async_trait::async_trait]
impl ChargeCategoricalUseCaseTrait for CHargeCategoricalUseCase {
    async fn charge_categorical_use_case(
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
        println!("llego a procesar los datos scatt");
        let features = Self::get_featurees_for_proyect(proyect_id.clone(), &conn).await?;
        println!("paso 1");
        let (data, set_map) = Self::process_files(data, proyect_id.clone(), &conn).await?;
        println!("paso 2");
        //ver si el dataframe esta cargado en memoria, si no cargarlo
        //primero el hashmap debe teenr soolo un dataframe en memoria asi que elimina todos en el
        //hashmap que no sea del dataframe
        let relate_requests = Self::process_categorical(features, data, conn).await?;
        println!("paso 3");
        Ok(JsonAdvanced(FileChargeResponse { ok: true }))
    }
}

impl CHargeCategoricalUseCase {
    pub async fn process_pair_columns(
        taget_col: FeaturesForProyect,
        category_col: FeaturesForProyect,
        dataframe: Arc<Py<PyAny>>,
    ) -> Result<RelateRequest<CategoricalToCategorical>, CsvError> {
        let python_process: String = Python::with_gil(|py| -> PyResult<String> {
            let module = PyModule::from_code(
                py,
                CString::new(PYTHON_ARCHIVE).unwrap().as_c_str(),
                CString::new("main.py").unwrap().as_c_str(),
                CString::new("embedded").unwrap().as_c_str(),
            )?
            .unbind();

            // obtener la función
            let func = module.getattr(py, "category_density_tables")?;

            // CSV en bytes
            // columnas desde set_map (HashSet<String>)
            let df = func
                .call1(
                    py,
                    (
                        dataframe.as_ref(),
                        PyString::new(py, &category_col.name),
                        PyString::new(py, &taget_col.name),
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

        //parser to serde_json
        let result: CategoricalContent = serde_json::from_str(&python_process).map_err(|e| {
            eprintln!("Python error: {:?}", e);
            CsvError::FileChargeError
        })?;
        let relate_request = RelateRequest::<CategoricalToCategorical>::builder()
            .from(&category_col.id.key().to_string())
            .to(&taget_col.id.key().to_string())
            .content(CategoricalToCategorical {
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
    pub async fn process_categorical(
        only_categorical: Vec<FeaturesForProyect>,
        data: Py<PyAny>,
        conn: Surreal<Any>,
    ) -> Result<(), CsvError> {
        println!("llego a procesar los datos scatt");
        let num_categorical = only_categorical.len();
        if num_categorical < 2 {
            return Ok(());
        }

        // Configuración óptima de hilos basada en los recursos del sistema
        let total_cores = num_cpus::get();
        let db_workers = (total_cores / 2).max(2); // Mitad de cores para DB
        let processing_workers = total_cores * 2; // Doble de cores para procesamiento

        println!(
            "Optimizando con {} cores totales: {} para DB, {} para procesamiento",
            total_cores, db_workers, processing_workers
        );

        let arc_conn = Arc::new(conn);

        // Canal con buffer optimizado
        let (tx, mut rx) =
            mpsc::channel::<RelateRequest<CategoricalToCategorical>>(processing_workers * 10);

        // Receptor optimizado para base de datos
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

                    // Limitar el número de tareas de DB concurrentes
                    if db_join_set.len() >= db_workers * 2 {
                        let _ = db_join_set.join_next().await;
                    }
                }

                // Esperar a que todas las tareas de DB terminen
                while let Some(res) = db_join_set.join_next().await {
                    if let Err(e) = res {
                        eprintln!("Error en tarea de DB: {:?}", e);
                    }
                }
            }
        });

        // Procesamiento paralelo optimizado
        let mut processing_join_set = JoinSet::new();
        let processing_semaphore = Arc::new(tokio::sync::Semaphore::new(processing_workers));
        let cats: Vec<_> = only_categorical.into_iter().collect();
        let data = Arc::new(data);

        for i in 0..num_categorical {
            for j in 0..num_categorical {
                if i == j {
                    continue;
                }
                let tx = tx.clone();
                let f1 = cats[i].clone();
                let f2 = cats[j].clone();
                let data = data.clone();
                let permit = processing_semaphore.clone().acquire_owned().await.unwrap();

                processing_join_set.spawn(async move {
                    let _permit = permit;

                    // Procesamiento en bloque para máximo rendimiento de Polars
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

        drop(tx); // cerrar canal, así el receiver sabe cuándo terminar
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
        //es csv validator
        if preload_file.get(0).unwrap().extension != "csv" {
            return Err(CsvError::InvalidFileType);
        }
        let mut files = preload_file; // asumiendo que ya es tuyo
        let file = files.remove(0);
        let bytes = file.file_data; // ahora sí tienes ownership
        let features = Self::get_featurees_for_proyect(proyect_id.clone(), &db).await?;
        let set_map = features
            .iter()
            .map(|x| x.name.clone())
            .collect::<HashSet<String>>();
        let body = data
            .get_data()
            .ok_or_else(|| CsvError::FileChargeError)?
            .clone();
        let separator = body.separator.unwrap_or(",".to_string());
        let df: Py<PyAny> = Python::attach(|py| -> PyResult<Py<PyAny>> {
            // compilar el script Python
            let module = PyModule::from_code(
                py,
                CString::new(PYTHON_ARCHIVE).unwrap().as_c_str(),
                CString::new("main.py").unwrap().as_c_str(),
                CString::new("embedded").unwrap().as_c_str(),
            )?
            .unbind();

            // obtener la función
            let func = module.getattr(py, "process_dataframe_from_csv_bytes")?;
            let py_separator_str = PyString::new(py, &separator).unbind();
            // CSV en bytes
            let py_csv_bytes = PyBytes::new(py, &bytes).unbind();

            // columnas desde set_map (HashSet<String>)
            let py_columns = PyList::new(py, set_map.clone().iter()).unwrap().unbind();

            // llamar a la función
            let df = func.call1(py, (py_csv_bytes, py_columns, py_separator_str))?;

            // de momento, solo devolvemos repr() del DataFrame
            Ok(df)
        })
        .map_err(|e| {
            eprintln!("Python error: {:?}", e);
            CsvError::FileChargeError
        })?;

        Ok((df, features))
    }
    async fn get_featurees_for_proyect(
        proyect_id: String,
        db: &Surreal<Any>,
    ) -> Result<Vec<FeaturesForProyect>, CsvError> {
        let params = json!({ "project" : format!("mst_proyect:{}", proyect_id) });
        let query = "RETURN (SELECT id,->mst_proyect_feature->(SELECT * FROM mst_feature where type_feature='Categorical').{name,id} as features from <record>$project).features[0];";
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
