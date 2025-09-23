use futures::SinkExt;
use polars::lazy::dsl::col as expr_col;
use polars::prelude::*;
use pyo3::{
    IntoPyObjectExt, Py, PyAny, PyResult, Python,
    types::{IntoPyDict, PyBytes, PyList, PyModule, PyString},
};
use rand::{Rng, seq::SliceRandom, thread_rng};
use rust_decimal::{Decimal, prelude::FromPrimitive};
use std::sync::Mutex as Mutexstd;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ffi::CString,
    sync::Arc,
};
use surrealdb::{Surreal, engine::any::Any};
use tokio::{
    sync::{Mutex, Semaphore, mpsc},
    task::{self, JoinSet},
};

use ac_struct_back::{
    import::macro_import::TableName,
    schemas::config::{
        feature::{
            feature::{CreateFeatureError, Feature},
            feature_type::FeatureType,
        },
        feature_to_feature::{
            feature_to_feature::FeatureToFeature, scatter_content::ScatterContent,
        },
        proyect::proyect::Proyect,
        proyect_feature::proyect_feature::ProyectFeature,
    },
    utils::domain::{
        query::{GraphBuilder, OneOrMany, Query, execute_select_query},
        relations::{RelateRequest, RelateRequestBuilder, execute_relate_query},
    },
};
use common::utils::ntex_private::extractors::{
    json::JsonAdvanced, multipart_extractor::MultipartData,
};
use minmaxlttb::{Lttb, LttbBuilder, LttbMethod, Point};
use polars::{error::PolarsResult, frame::DataFrame, io::SerReader, prelude::*};

use crate::{
    PYTHON_ARCHIVE,
    modules::app_charge::{
        domain::{
            data::charge_dto::ChargeDto,
            models::proyect_desnormalized::ProyectDesnormalized,
            response::file_charge_response::FileChargeResponse,
            use_case::charge_file_use_case::{ChargeFileUseCase, ChargeFileUseCaseTrait},
        },
        infrastructure::use_case::impl_get_all_data_use_case::DATA_FRAMES,
    },
    try_get_surreal_pool,
    utils::{charge_models::void_struct::VoidStruct, errors::csv_error::CsvError},
};
#[async_trait::async_trait]
impl ChargeFileUseCaseTrait for ChargeFileUseCase {
    async fn charge_file(
        dto: MultipartData<ChargeDto>,
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
        let proyect = Self::get_proyect_by_id(proyect_id.clone(), &conn).await?;
        //get first proyect
        println!("llego a apasa el proyect");
        let proyect = proyect.into_iter().next().unwrap();

        let (data, hashmap) = Self::process_files(dto).await?;
        //ver si el dataframe esta cargado en memoria, si no cargarlo
        //primero el hashmap debe teenr soolo un dataframe en memoria asi que elimina todos en el
        //hashmap que no sea del dataframe

        fn keep_only_key(key: &str, new_df: DataFrame) {
            let mut map = DATA_FRAMES.write().unwrap();

            // Si la key no existe, la agregamos
            map.entry(key.to_string()).or_insert(new_df);

            // Eliminamos todas las keys que no sean la especificada
            map.retain(|k, _| k == key);
        }
        println!("llego a procesar los datos");
        let features = Self::profile_dataframe(&data, &hashmap).map_err(|e| {
            println!("{:?}", e);
            CsvError::InvalidFileContent
        })?;
        println!("features: {:?}", features);
        //create features
        let created = Self::create_features_and_proyects(features, proyect_id, &conn).await?;

        Ok(JsonAdvanced(FileChargeResponse { ok: true }))
    }
}
fn downsample_with_density(points: &[(f64, f64)], target: usize) -> Vec<ScatterContent> {
    // Calcular rangos
    let (x_min, x_max) = points
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), &(x, _)| {
            (min.min(x), max.max(x))
        });
    let (y_min, y_max) = points
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), &(_, y)| {
            (min.min(y), max.max(y))
        });

    let x_range = x_max - x_min;
    let y_range = y_max - y_min;

    // Crear bins y calcular densidad
    let n_bins = 50;
    let bin_size_x = if x_range == 0.0 {
        1.0
    } else {
        x_range / n_bins as f64
    };
    let bin_size_y = if y_range == 0.0 {
        1.0
    } else {
        y_range / n_bins as f64
    };

    let mut density_map: Arc<Mutexstd<HashMap<(i32, i32), usize>>> =
        Arc::new(Mutexstd::new(HashMap::new()));
    let mut point_bins: Arc<Mutexstd<Vec<((i32, i32), (f64, f64))>>> =
        Arc::new(Mutexstd::new(Vec::with_capacity(points.len())));
    let inb_bin_x = 1.0 / bin_size_x;
    let inb_bin_y = 1.0 / bin_size_y;

    // Paso 1: llenar bins en paralelo
    points.par_iter().for_each(|(x, y)| {
        let bin_x = ((x - x_min) * inb_bin_x) as i32;
        let bin_y = ((y - y_min) * inb_bin_y) as i32;

        {
            let mut map = density_map.lock().unwrap();
            *map.entry((bin_x, bin_y)).or_default() += 1;
        }
        {
            let mut bins = point_bins.lock().unwrap();
            bins.push(((bin_x, bin_y), (*x, *y)));
        }
    });

    let density_map = Arc::try_unwrap(density_map).unwrap().into_inner().unwrap();
    let max_density = *density_map.values().max().unwrap_or(&1) as f64;

    let points_bin = Arc::try_unwrap(point_bins).unwrap().into_inner().unwrap();
    use rayon::prelude::*;
    // Paso 2: muestreo paralelo con probabilidad
    let sampled_points: Vec<(f64, f64)> = points_bin
        .par_iter()
        .fold(
            || Vec::new(),
            |mut local_vec, (bin, (x, y))| {
                let mut rng = rand::thread_rng();
                let density = *density_map.get(bin).unwrap_or(&1) as f64;

                let center_x = (x_min + x_max) / 2.0;
                let center_y = (y_min + y_max) / 2.0;
                let dist = ((x - center_x).powi(2) + (y - center_y).powi(2)).sqrt();

                let prob = (target as f64) / (points.len() as f64 * (density / max_density).sqrt())
                    * (1.0 + dist / (x_range.max(y_range)));

                if density <= 2.0 {
                    local_vec.push((*x, *y));
                } else if rng.random::<f64>() < prob {
                    local_vec.push((*x, *y));
                }
                local_vec
            },
        )
        .reduce(
            || Vec::new(),
            |mut a, mut b| {
                a.append(&mut b);
                a
            },
        );

    // Paso 3: ajustar tamaño final
    let mut rng = rand::thread_rng();
    let mut sampled_points = sampled_points;
    if sampled_points.len() > target {
        sampled_points.shuffle(&mut rng);
        sampled_points.truncate(target);
    } else if sampled_points.len() < 500 {
        let mut remaining: Vec<_> = points
            .iter()
            .filter(|p| !sampled_points.contains(p))
            .cloned()
            .collect();
        remaining.shuffle(&mut rng);
        sampled_points.extend(remaining.into_iter().take(500 - sampled_points.len()));
    }

    sampled_points
        .into_iter()
        .map(|(x, y)| ScatterContent { x, y })
        .collect()
}

//fn downsample_with_density(points: &[(f64, f64)], target: usize) -> Vec<ScatterContent> {
//    use rand::Rng;
//    use std::collections::HashMap;
//
//    // Calcular rangos
//    let (x_min, x_max) = points
//        .iter()
//        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), &(x, _)| {
//            (min.min(x), max.max(x))
//        });
//
//    let (y_min, y_max) = points
//        .iter()
//        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), &(_, y)| {
//            (min.min(y), max.max(y))
//        });
//
//    let x_range = x_max - x_min;
//    let y_range = y_max - y_min;
//
//    // Crear bins y calcular densidad
//    let n_bins = 50;
//    let bin_size_x = if x_range == 0.0 {
//        1.0
//    } else {
//        x_range / n_bins as f64
//    };
//    let bin_size_y = if y_range == 0.0 {
//        1.0
//    } else {
//        y_range / n_bins as f64
//    };
//
//    let mut density_map: HashMap<(i32, i32), usize> = HashMap::new();
//    let mut point_bins: Vec<((i32, i32), (f64, f64))> = Vec::with_capacity(points.len());
//    let inb_bin_x = 1.0 / bin_size_x;
//    let inb_bin_y = 1.0 / bin_size_y;
//
//    for &(x, y) in points {
//        let bin_x = ((x - x_min) * inb_bin_x) as i32;
//        let bin_y = ((y - y_min) * inb_bin_y) as i32;
//
//        *density_map.entry((bin_x, bin_y)).or_insert(0) += 1;
//        point_bins.push(((bin_x, bin_y), (x, y)));
//    }
//
//    // Encontrar densidad máxima
//    let max_density = *density_map.values().max().unwrap_or(&1) as f64;
//
//    // Muestreo probabilístico
//    let mut rng = rand::thread_rng();
//    let mut sampled_points = Vec::with_capacity(target);
//
//    for (bin, (x, y)) in point_bins {
//        let density = *density_map.get(&bin).unwrap_or(&1) as f64;
//
//        let center_x = (x_min + x_max) / 2.0;
//        let center_y = (y_min + y_max) / 2.0;
//        let dist = ((x - center_x).powi(2) + (y - center_y).powi(2)).sqrt();
//
//        // dar más probabilidad a puntos alejados
//        let prob = (target as f64) / (points.len() as f64 * (density / max_density).sqrt())
//            * (1.0 + dist / (x_range.max(y_range)));
//        if density <= 2.0 {
//            sampled_points.push((x, y));
//        } else if rng.random::<f64>() < prob {
//            sampled_points.push((x, y));
//        }
//    }
//
//    // Ajustar tamaño final
//    if sampled_points.len() > target {
//        sampled_points.shuffle(&mut rng);
//        sampled_points.truncate(target);
//    } else if sampled_points.len() < 500 {
//        // Mínimo de puntos
//        let mut remaining: Vec<_> = points
//            .iter()
//            .filter(|p| !sampled_points.contains(p))
//            .cloned()
//            .collect();
//        remaining.shuffle(&mut rng);
//        sampled_points.extend(remaining.into_iter().take(500 - sampled_points.len()));
//    }
//
//    sampled_points
//        .into_iter()
//        .map(|(x, y)| ScatterContent { x, y })
//        .collect()
//}
impl ChargeFileUseCase {
    async fn process_feature_pair(
        f1: Feature,
        f2: Feature,
        data: LazyFrame,
    ) -> Result<RelateRequestBuilder<FeatureToFeature>, CsvError> {
        println!("processing");
        println!("f1 name: {}", f1.name);
        println!("f2 name: {}", f2.name);
        let f1_name = &f1.name;
        let f2_name = &f2.name;
        let f1_id = f1.id.as_ref().unwrap().key().to_string();
        let f2_id = f2.id.as_ref().unwrap().key().to_string();
        let lazy_df = data
            .clone()
            .select([col(f1_name), col(f2_name)])
            .drop_nulls(None)
            .sort(
                [f1_name.as_str()], // ✅ pasamos &str, Polars se encarga
                SortMultipleOptions::new()
                    .with_order_descending_multi([false])
                    .with_nulls_last(true),
            );
        let pair_df = lazy_df.collect().map_err(|e| {
            println!("{:?}", e);
            CsvError::InvalidFileContent
        })?;
        let x_ca = pair_df
            .column(f1_name)
            .map_err(|e| {
                println!("{:?}", e);
                CsvError::InvalidFileContent
            })?
            .cast(&DataType::Float64)
            .map_err(|e| {
                println!("{:?}", e);
                CsvError::InvalidFileContent
            })?;
        let x_ca = x_ca.f64().map_err(|e| {
            println!("{:?}", e);
            CsvError::InvalidFileContent
        })?;
        let y_ca = pair_df
            .column(f2_name)
            .map_err(|e| {
                println!("{:?}", e);
                CsvError::InvalidFileContent
            })?
            .cast(&DataType::Float64)
            .map_err(|e| {
                println!("{:?}", e);
                CsvError::InvalidFileContent
            })?;
        let y_ca = y_ca.f64().map_err(|e| {
            println!("{:?}", e);
            CsvError::InvalidFileContent
        })?;

        let mut points: Vec<(f64, f64)> = Vec::with_capacity(pair_df.height());
        for (x, y) in x_ca.into_no_null_iter().zip(y_ca.into_no_null_iter()) {
            if !x.is_nan() && !y.is_nan() {
                points.push((x, y));
            }
        }
        points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        //points.dedup_by(|a, b| a.x() == b.x());
        let content_scatter = if points.len() > 2000 {
            downsample_with_density(&points, 1000)
        } else {
            points
                .iter()
                .map(|(x, y)| ScatterContent { x: *x, y: *y })
                .collect()
        };

        let relate_request = RelateRequest::<FeatureToFeature>::builder()
            .from(&f1_id)
            .to(&f2_id)
            .content(FeatureToFeature {
                content_scatter,
                ..Default::default()
            })
            .map_err(|e| {
                println!("{:?}", e);
                CsvError::FileChargeError
            })?
            .get_owned();
        println!("done");
        Ok(relate_request)
    }

    pub async fn process_scatterplots(
        continuous: Vec<&Feature>,
        only_continuous: Vec<&Feature>,
        data: LazyFrame,
        conn: Surreal<Any>,
    ) -> Result<(), CsvError> {
        println!("llego a procesar los datos scatt");
        let num_continuous = continuous.len();
        let only_value = only_continuous.len();
        if num_continuous < 2 {
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
            mpsc::channel::<RelateRequest<FeatureToFeature>>(processing_workers * 10);

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

        for i in 0..only_value {
            for j in i..num_continuous {
                let tx = tx.clone();
                let f1 = only_continuous[i].clone();
                let f2 = continuous[j].clone();
                let data = data.clone();
                let permit = processing_semaphore.clone().acquire_owned().await.unwrap();

                processing_join_set.spawn(async move {
                    let _permit = permit;

                    // Procesamiento en bloque para máximo rendimiento de Polars
                    match tokio::task::spawn_blocking(move || {
                        Self::process_feature_pair(f1, f2, data)
                    })
                    .await
                    .unwrap()
                    .await
                    {
                        Ok(relate_request) => {
                            if let Err(e) = tx.send(relate_request.build().unwrap()).await {
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
        mut data: MultipartData<ChargeDto>,
    ) -> Result<(Py<PyAny>, HashMap<String, FeatureType>), CsvError> {
        println!("data {:?}", data.get_data());
        let body = data
            .get_data()
            .ok_or_else(|| CsvError::FileChargeError)?
            .clone();
        println!("body: {:?}", body);
        let set_map = body
            .fields
            .iter()
            .map(|x| (x.name.clone()))
            .collect::<HashSet<String>>();
        let hashmap = body
            .fields
            .iter()
            .map(|x| (x.name.clone(), x._type.clone()))
            .collect::<HashMap<String, FeatureType>>();
        let preload_file = data.take_files().ok_or_else(|| CsvError::FileChargeError)?;
        if preload_file.is_empty() || preload_file.len() > 1 {
            return Err(CsvError::FileChargeError);
        }
        //es csv validator
        if preload_file.get(0).unwrap().extension != "csv" {
            return Err(CsvError::InvalidFileType);
        }
        let body = data
            .get_data()
            .ok_or_else(|| CsvError::FileChargeError)?
            .clone();
        let separator = body.separator.unwrap_or(",".to_string());
        let mut files = preload_file; // asumiendo que ya es tuyo
        let file = files.remove(0);
        let bytes = file.file_data; // ahora sí tienes ownership
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

            // CSV en bytes
            let py_csv_bytes = PyBytes::new(py, &bytes).unbind();

            // columnas desde set_map (HashSet<String>)
            let py_columns = PyList::new(py, set_map.iter()).unwrap().unbind();
            let py_separator_str = PyString::new(py, &separator).unbind();
            // llamar a la función
            let df = func.call1(py, (py_csv_bytes, py_columns, py_separator_str))?;

            // de momento, solo devolvemos repr() del DataFrame
            Ok(df)
        })
        .map_err(|e| {
            eprintln!("Python error: {:?}", e);
            CsvError::FileChargeError
        })?;
        Ok((df, hashmap))
    }

    async fn take_columns(
        data: DataFrame,
        collumns: HashSet<String>,
    ) -> Result<DataFrame, CsvError> {
        let collumns = collumns
            .into_iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>();
        let collumns = collumns.iter().map(|x| x.as_str()).collect::<Vec<&str>>();
        let df = data.select(collumns).map_err(|e| {
            println!("{:?}", e);
            CsvError::InvalidFileContent
        })?;
        Ok(df)
    }
    pub fn profile_dataframe(
        df: &Py<PyAny>,
        types: &HashMap<String, FeatureType>,
    ) -> Result<(Vec<Feature>), String> {
        // Precompute statistics for continuous columns using expressions
        let continuous_cols: Vec<String> = types
            .iter()
            .filter(|(_, t)| matches!(**t, FeatureType::Continuous))
            .map(|(name, _)| name.clone())
            .collect();
        let stats_df: Py<PyAny> = Python::attach(|py| -> PyResult<Py<PyAny>> {
            // compilar el script Python
            let module = PyModule::from_code(
                py,
                CString::new(PYTHON_ARCHIVE).unwrap().as_c_str(),
                CString::new("main.py").unwrap().as_c_str(),
                CString::new("embedded").unwrap().as_c_str(),
            )?
            .unbind();

            println!("paso 0");
            // obtener la función compute_stats
            let func = module.getattr(py, "compute_stats")?;

            //
            // columnas continuas (Rust Vec<String> -> Python list)
            println!("paso 1");
            let py_columns = PyList::new(py, continuous_cols.iter()).unwrap().unbind();

            // llamar a compute_stats(df, continuous_cols)
            println!("paso 2");
            let stats = func.call1(py, (&df, py_columns))?;
            println!("paso 3");
            Ok(stats)
        })
        .map_err(|e| e.to_string())?;
        println!("paso duration");
        //volter types a HashmapStrnig,String> con serde,
        //volver types a Hashmap<String,String>
        let types: HashMap<String, String> = types
            .into_iter()
            .map(|(k, v)| {
                let mut feature_type = match v {
                    FeatureType::Continuous => "Continuous",
                    FeatureType::Categorical => "Categorical",
                };
                (k.to_string(), feature_type.to_string())
            })
            .collect();
        println!("types: {:?}", types);

        let value = Python::attach(|py| -> PyResult<String> {
            let module = PyModule::from_code(
                py,
                CString::new(PYTHON_ARCHIVE).unwrap().as_c_str(),
                CString::new("main.py").unwrap().as_c_str(),
                CString::new("embedded").unwrap().as_c_str(),
            )?
            .unbind();
            let func = module.getattr(py, "compute_features")?;
            let py_types = types.clone().into_py_dict(py)?.unbind();
            let result = func
                .call1(py, (&df, &stats_df, py_types))?
                .into_any()
                .extract(py);
            result
        })
        .map_err(|e| e.to_string())?;
        println!("value: {}", value);
        let features: Vec<Feature> = serde_json::from_str(&value).map_err(|e| e.to_string())?;

        Ok((features))
    }
    pub async fn get_proyect_by_id(
        proyect_id: String,
        db: &Surreal<Any>,
    ) -> Result<Vec<ProyectDesnormalized>, CsvError> {
        let graph_expression = GraphBuilder::new()
            .out(ProyectFeature::table_name())
            .out(Feature::table_name())
            .project_object()
            .build()
            .build();
        let exist: OneOrMany<ProyectDesnormalized> = execute_select_query(
            Query::<Proyect>::new()
                .from(Some(&proyect_id), false)
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
                    if let Some(proyect) = val.first() {
                        if !proyect.fields.is_empty() {
                            return Err(CsvError::FileChargeError);
                        }
                    }
                    Ok(val)
                }
            }
        }
    }
    async fn create_features_and_proyects(
        features: Vec<Feature>,
        proyect_id: String,
        db: &Surreal<Any>,
    ) -> Result<Vec<Feature>, CsvError> {
        let created: Vec<Feature> = db
            .insert(Feature::table_name())
            .content(features)
            .await
            .map_err(|e| {
                println!("{:?}", e);
                CsvError::FileChargeError
            })?;

        let v: Vec<String> = created
            .iter()
            .map(|x| x.id.as_ref().unwrap().key().to_string())
            .collect();
        let relates: Vec<ProyectFeature> = execute_relate_query(
            RelateRequest::<ProyectFeature>::builder()
                .from(&proyect_id)
                .to_vec(v.iter().map(|x| x.as_str()).collect::<Vec<&str>>())
                .content(ProyectFeature {
                    ..Default::default()
                })
                .map_err(|e| {
                    println!("{:?}", e);
                    CsvError::FileChargeError
                })?
                .get_owned(),
            db,
            false,
        )
        .await
        .map_err(|e: CreateFeatureError| {
            println!("{:?}", e);
            CsvError::FileChargeError
        })?;
        Ok(created)
    }
}
