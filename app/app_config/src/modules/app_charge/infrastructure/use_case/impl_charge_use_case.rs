use futures::SinkExt;
use polars::lazy::dsl::col as expr_col;
use polars::prelude::*;
use rand::{Rng, seq::SliceRandom, thread_rng};
use rust_decimal::{Decimal, prelude::FromPrimitive};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
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
    modules::app_charge::domain::{
        data::charge_dto::ChargeDto,
        models::proyect_desnormalized::ProyectDesnormalized,
        response::file_charge_response::FileChargeResponse,
        use_case::charge_file_use_case::{ChargeFileUseCase, ChargeFileUseCaseTrait},
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
        println!("llego a procesar los datos");
        let count = (&data).height();
        let (features, data) = Self::profile_dataframe(&data, &hashmap).map_err(|e| {
            println!("{:?}", e);
            CsvError::InvalidFileContent
        })?;
        println!("features: {:?}", features);
        //create features
        let created = Self::create_features_and_proyects(features, proyect_id, &conn).await?;

        let continuous: Vec<&Feature> = created
            .iter()
            .filter(|f| matches!(f.type_feature, FeatureType::Continuous))
            .collect();
        Self::process_scatterplots(continuous, data, conn).await?;

        Ok(JsonAdvanced(FileChargeResponse { ok: true }))
    }
}

impl ChargeFileUseCase {
    async fn process_feature_pair(
        f1: Feature,
        f2: Feature,
        data: LazyFrame,
    ) -> Result<RelateRequestBuilder<FeatureToFeature>, CsvError> {
        println!("processing");
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

        let content_scatter: Vec<ScatterContent>;
        let min_points = 500; // mínimo para datasets pequeños
        let max_points = 10000; // máximo para datasets grandes
        let target_ratio = 0.03; // queremos tomar ~3% de los puntos

        let mut target = (points.len() as f64 * target_ratio).ceil() as usize;
        target = target.clamp(min_points, max_points);
        let scatter_content: Vec<ScatterContent>;

        if points.len() > 10000 {
            let lazy_df = pair_df.clone().lazy();
            let stats = lazy_df
                .clone()
                .select([
                    col(f1_name).cast(DataType::Float64).min().alias("x_min"),
                    col(f1_name).cast(DataType::Float64).max().alias("x_max"),
                    col(f2_name).cast(DataType::Float64).min().alias("y_min"),
                    col(f2_name).cast(DataType::Float64).max().alias("y_max"),
                ])
                .collect()
                .map_err(|e| {
                    println!("{:?}", e);
                    CsvError::InvalidFileContent
                })?;

            let x_min = stats
                .column("x_min")
                .map_err(|e| {
                    println!("{:?}", e);
                    CsvError::InvalidFileContent
                })?
                .f64()
                .map_err(|e| {
                    println!("{:?}", e);
                    CsvError::InvalidFileContent
                })?
                .get(0)
                .unwrap_or(0.0);
            let x_max = stats
                .column("x_max")
                .map_err(|e| {
                    println!("{:?}", e);
                    CsvError::InvalidFileContent
                })?
                .f64()
                .map_err(|e| {
                    println!("{:?}", e);
                    CsvError::InvalidFileContent
                })?
                .get(0)
                .unwrap_or(1.0);
            let y_min = stats
                .column("y_min")
                .map_err(|e| {
                    println!("{:?}", e);
                    CsvError::InvalidFileContent
                })?
                .f64()
                .map_err(|e| {
                    println!("{:?}", e);
                    CsvError::InvalidFileContent
                })?
                .get(0)
                .unwrap_or(0.0);
            let y_max = stats
                .column("y_max")
                .map_err(|e| {
                    println!("{:?}", e);
                    CsvError::InvalidFileContent
                })?
                .f64()
                .map_err(|e| {
                    println!("{:?}", e);
                    CsvError::InvalidFileContent
                })?
                .get(0)
                .unwrap_or(1.0);

            let x_range = x_max - x_min;
            let y_range = y_max - y_min;
            let bin_size_x = if x_range == 0.0 { 1.0 } else { x_range / 50.0 };

            let bin_size_y = if y_range == 0.0 { 1.0 } else { y_range / 50.0 };
            let expr = (expr_col(f1_name).cast(DataType::Float64) - lit(x_min)) / lit(bin_size_x);
            let binned_df = lazy_df
                .with_column(expr.floor().cast(DataType::Int32).alias("bin_x"))
                .with_column(
                    ((col(f2_name).cast(DataType::Float64) - lit(y_min)) / lit(bin_size_y))
                        .floor()
                        .cast(DataType::Int32)
                        .alias("bin_y"),
                )
                .group_by(["bin_x", "bin_y"])
                .agg([col(f1_name).count().fill_null(lit(1)).alias("density")])
                .collect()
                .map_err(|e| {
                    println!("{:?}", e);
                    CsvError::InvalidFileContent
                })?;
            let joined_df = pair_df
                .clone()
                .lazy()
                .with_column(
                    ((col(f1_name).cast(DataType::Float64) - lit(x_min)) / lit(bin_size_x))
                        .floor()
                        .cast(DataType::Int32)
                        .alias("bin_x"),
                )
                .with_column(
                    ((col(f2_name).cast(DataType::Float64) - lit(y_min)) / lit(bin_size_y))
                        .floor()
                        .cast(DataType::Int32)
                        .alias("bin_y"),
                )
                .join_builder()
                .with(binned_df.lazy()) // tabla derecha
                .how(JoinType::Left) // join izquierdo
                .left_on(&[col("bin_x"), col("bin_y")]) // columnas izquierda
                .right_on(&[col("bin_x"), col("bin_y")]) // columnas derecha
                .finish()
                .collect()
                .map_err(|e| {
                    println!("{:?}", e);
                    CsvError::InvalidFileContent
                })?;

            // Extraer densidades
            let density_col = joined_df
                .column("density")
                .map_err(|e| {
                    println!("{:?}", e);
                    CsvError::InvalidFileContent
                })?
                .u32()
                .map_err(|e| {
                    println!("{:?}", e);
                    CsvError::InvalidFileContent
                })?;
            let max_density = density_col.max().unwrap_or(1) as f64;

            // Muestreo probabilístico basado en densidad inversa
            let mut rng = thread_rng();
            let mut sampled_points = Vec::new();
            let x_series = joined_df
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

            let x_values = x_series
                .f64()
                .map_err(|e| {
                    println!("{:?}", e);
                    CsvError::InvalidFileContent
                })?
                .into_no_null_iter();

            let y_series = joined_df
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

            let y_values = y_series
                .f64()
                .map_err(|e| {
                    println!("{:?}", e);
                    CsvError::InvalidFileContent
                })?
                .into_no_null_iter();
            for (idx, (x, y)) in x_values.zip(y_values).enumerate() {
                let density = density_col.get(idx).unwrap_or(1) as f64;
                let prob = (target as f64) / (points.len() as f64 * (density / max_density).sqrt());
                if rng.random::<f64>() < prob {
                    sampled_points.push((x.clone(), y.clone()));
                }
            }
            if sampled_points.len() > target {
                sampled_points.shuffle(&mut rng);
                sampled_points.truncate(target);
            } else if sampled_points.len() < min_points {
                // Si no alcanzamos min_points, tomamos más puntos aleatoriamente
                let mut remaining = points.clone();
                remaining.shuffle(&mut rng);
                let needed = min_points - sampled_points.len();
                sampled_points.extend(remaining.into_iter().take(needed).map(|a| (a.x(), a.y())));
            }
            content_scatter = sampled_points
                .iter()
                .map(|p| ScatterContent { x: p.0, y: p.1 })
                .collect()
        } else {
            content_scatter = points
                .iter()
                .map(|x| ScatterContent { x: x.x(), y: x.y() })
                .collect();
        }
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

    async fn process_scatterplots(
        continuous: Vec<&Feature>,
        data: LazyFrame,
        conn: Surreal<Any>,
    ) -> Result<(), CsvError> {
        let num_continuous = continuous.len();
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

        for i in 0..num_continuous {
            for j in (i + 1)..num_continuous {
                let tx = tx.clone();
                let f1 = continuous[i].clone();
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
    async fn process_files(
        mut data: MultipartData<ChargeDto>,
    ) -> Result<(DataFrame, HashMap<String, FeatureType>), CsvError> {
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
        let mut files = preload_file; // asumiendo que ya es tuyo
        let file = files.remove(0);
        let bytes = file.file_data; // ahora sí tienes ownership
        let cursor = std::io::Cursor::new((&bytes).as_ref());
        let parse_opts = CsvParseOptions {
            separator: b',',             // lo más común: coma como separador
            quote_char: Some(b'"'),      // campos entre comillas dobles
            eol_char: b'\n',             // salto de línea estándar
            encoding: CsvEncoding::Utf8, // hoy en día casi todo es UTF-8
            null_values: None,           // NULLs detectados por defecto (vacíos)
            missing_is_null: true,       // celdas vacías = null
            truncate_ragged_lines: true, // si faltan columnas, completa con nulls
            comment_prefix: None,        // sin soporte de comentarios
            try_parse_dates: true,       // intenta detectar fechas
            decimal_comma: false,        // por defecto: decimal con punto (.)
        };

        // Configuración general
        let options = CsvReadOptions {
            has_header: true,                // primera fila = nombres de columnas
            rechunk: true,                   // junta chunks para mejor rendimiento
            n_threads: None,                 // None = usa todos los cores
            low_memory: false,               // false = más rápido, usa más RAM
            chunk_size: 1_000_000,           // procesa en chunks grandes (ajustable)
            infer_schema_length: Some(1000), // analiza primeras 1000 filas para tipos
            ignore_errors: false,            // si hay errores, detiene
            parse_options: Arc::new(parse_opts),
            ..Default::default()
        };
        let df = CsvReader::new(cursor)
            .with_options(options)
            .finish()
            .map_err(|e| {
                println!("{:?}", e);
                CsvError::FileChargeError
            })?;
        let final_dt = Self::take_columns(df, set_map).await?;
        Ok((final_dt, hashmap))
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
        df: &DataFrame,
        types: &HashMap<String, FeatureType>,
    ) -> PolarsResult<(Vec<Feature>, LazyFrame)> {
        let total_rows = df.height() as f64;
        let mut features = Vec::with_capacity(df.width());
        let lazy_df = df.clone().lazy();

        // Precompute statistics for continuous columns using expressions
        let continuous_cols: Vec<String> = types
            .iter()
            .filter(|(_, t)| matches!(**t, FeatureType::Continuous))
            .map(|(name, _)| name.clone())
            .collect();
        let stats_df = if !continuous_cols.is_empty() {
            lazy_df
                .clone()
                .select(
                    continuous_cols
                        .iter()
                        .flat_map(|name| {
                            let col = col(name).cast(DataType::Float64);
                            vec![
                                col.clone().min().alias(&format!("{}_min", name)),
                                col.clone().max().alias(&format!("{}_max", name)),
                                col.clone().mean().alias(&format!("{}_mean", name)),
                                col.clone().std(1).alias(&format!("{}_std", name)),
                                col.clone()
                                    .quantile(lit(0.5), QuantileMethod::Linear)
                                    .alias(&format!("{}_median", name)),
                                col.clone()
                                    .quantile(lit(0.25), QuantileMethod::Linear)
                                    .alias(&format!("{}_q25", name)),
                                col.clone()
                                    .quantile(lit(0.75), QuantileMethod::Linear)
                                    .alias(&format!("{}_q75", name)),
                            ]
                        })
                        .collect::<Vec<_>>(),
                )
                .collect()?
        } else {
            DataFrame::empty()
        };
        println!("paso duration");

        for name in df.get_column_names() {
            let col = df.column(name)?;
            let mut feature = Feature::default();
            feature.name = name.to_string();
            feature.count = df.height() as u64;

            // Missing %
            let n_null = col.null_count() as f64;
            feature.misses_percent = ((n_null / total_rows) * 100.0) as u32;

            // Cardinality
            feature.cardinality = col.n_unique()? as u64;

            // Check feature type from the map
            let feat_type = types
                .get(name.as_str())
                .cloned()
                .unwrap_or(FeatureType::Categorical);
            feature.type_feature = feat_type.clone();

            match feat_type {
                FeatureType::Continuous => {
                    // Use precomputed statistics from stats_df

                    if let Ok(col_min) = stats_df.column(&format!("{}_min", name)) {
                        if let Ok(val) = col_min.get(0) {
                            if let Ok(v) = val.try_extract::<f64>() {
                                feature.min = Decimal::from_f64(v).unwrap_or(Decimal::new(0, 0));
                            }
                        }
                    }
                    if let Ok(col_max) = stats_df.column(&format!("{}_max", name)) {
                        if let Ok(val) = col_max.get(0) {
                            if let Ok(v) = val.try_extract::<f64>() {
                                feature.max = Decimal::from_f64(v).unwrap_or(Decimal::new(0, 0));
                            }
                        }
                    }
                    if let Ok(col_mean) = stats_df.column(&format!("{}_mean", name)) {
                        if let Ok(val) = col_mean.get(0) {
                            if let Ok(v) = val.try_extract::<f64>() {
                                feature.mean = Decimal::from_f64(v).unwrap_or(Decimal::new(0, 0));
                            }
                        }
                    }
                    if let Ok(col_std) = stats_df.column(&format!("{}_std", name)) {
                        if let Ok(val) = col_std.get(0) {
                            if let Ok(v) = val.try_extract::<f64>() {
                                feature.standard_deviation =
                                    Decimal::from_f64(v).unwrap_or(Decimal::new(0, 0));
                            }
                        }
                    }
                    if let Ok(col_median) = stats_df.column(&format!("{}_median", name)) {
                        if let Ok(val) = col_median.get(0) {
                            if let Ok(v) = val.try_extract::<f64>() {
                                feature.median = Decimal::from_f64(v).unwrap_or(Decimal::new(0, 0));
                            }
                        }
                    }
                    if let Ok(col_q25) = stats_df.column(&format!("{}_q25", name)) {
                        if let Ok(val) = col_q25.get(0) {
                            if let Ok(v) = val.try_extract::<f64>() {
                                feature.per_quartil =
                                    Decimal::from_f64(v).unwrap_or(Decimal::new(0, 0));
                            }
                        }
                    }
                    if let Ok(col_q75) = stats_df.column(&format!("{}_q75", name)) {
                        if let Ok(val) = col_q75.get(0) {
                            if let Ok(v) = val.try_extract::<f64>() {
                                feature.tertile =
                                    Decimal::from_f64(v).unwrap_or(Decimal::new(0, 0));
                            }
                        }
                    }
                    // Distribución (histograma)
                    let data_type = DataType::Float64;
                    let tertile = feature.tertile.clone();
                    let per_quartil = feature.per_quartil.clone();
                    let iqr = per_quartil.clone() - tertile.clone();
                    let col_f64 = col.cast(&data_type)?.f64()?.clone();

                    let mut values: Vec<f64> = col_f64.into_no_null_iter().collect();
                    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    let n_data = values.len();
                    use rust_decimal::prelude::ToPrimitive;

                    let bin_width = 2.0 * iqr.to_f64().unwrap() / (n_data as f64).cbrt();

                    let min = *values
                        .iter()
                        .min_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap();
                    let max = *values
                        .iter()
                        .max_by(|a, b| a.partial_cmp(b).unwrap())
                        .unwrap();
                    let n_bins = if bin_width > 0.0 {
                        ((max - min) / bin_width).ceil() as usize
                    } else {
                        1.max((1.0 + (n_data as f64)).log2().ceil() as usize) // fallback a Sturges si IQR=0
                    };
                    if !values.is_empty() {
                        let width = (max - min) / n_bins as f64;

                        let mut bins = Vec::with_capacity(n_bins);
                        let mut counts = vec![0f64; n_bins];
                        let mut intervals = Vec::with_capacity(n_bins);

                        for i in 0..n_bins {
                            bins.push((min + i as f64 * width).to_string());
                            let start = min + i as f64 * width;
                            let end = start + width;
                            //intervals Vec<Vec<f64>> = vec![vec![min, max], vec![min+width, max+width], vec![min+2*width, max+2*width]]
                            intervals.push(vec![start, end]);
                        }

                        for &v in &values {
                            let idx = ((v - min) / width).floor() as usize;
                            let idx = if idx >= n_bins { n_bins - 1 } else { idx };
                            counts[idx] += 1f64;
                        }
                        feature.distribution_intervals = intervals;

                        feature.distribution_bins = bins;
                        feature.distribution_counts = counts;
                    }
                }
                FeatureType::Categorical => {
                    // Value counts with sorting and parallel processing
                    let expr_name = expr_col(PlSmallStr::from_str(&name));
                    let vc = lazy_df
                        .clone()
                        .group_by_stable([expr_name.clone()])
                        .agg([expr_col(PlSmallStr::from_str(&name))
                            .count()
                            .alias("counts")])
                        .sort(["counts"], SortMultipleOptions::new()) // descendente
                        .collect()?;
                    if vc.height() > 0 {
                        let values = vc.column(col.name())?;
                        let counts = vc.column("counts")?;
                        let values = values.cast(&DataType::String)?;

                        feature.distribution_bins = values
                            .str()?
                            .into_iter()
                            .filter_map(|opt_s| opt_s.map(|s| s.to_string()))
                            .collect();
                        feature.distribution_counts = match counts.dtype() {
                            DataType::UInt64 => counts
                                .u64()?
                                .into_iter()
                                .filter_map(|opt| opt.map(|v| v as f64))
                                .collect(),
                            DataType::UInt32 => counts
                                .u32()?
                                .into_iter()
                                .filter_map(|opt| opt.map(|v| v as f64))
                                .collect(),
                            DataType::Int64 => counts
                                .i64()?
                                .into_iter()
                                .filter_map(|opt| opt.map(|v| v as f64))
                                .collect(),
                            DataType::Int32 => counts
                                .i32()?
                                .into_iter()
                                .filter_map(|opt| opt.map(|v| v as f64))
                                .collect(),
                            _ => {
                                // Fallback: intentar extraer cualquier número como f64
                                counts.f64()?.into_iter().filter_map(|opt| opt).collect()
                            }
                        };

                        if let Ok(mode_val) = values.get(0) {
                            feature.mode = mode_val.to_string().trim_matches('"').to_string();
                            if let Ok(mode_freq) = counts.get(0).unwrap().try_extract::<u64>() {
                                feature.mode_frequency = mode_freq;
                                feature.mode_percent =
                                    ((mode_freq as f64 / total_rows) * 100.0) as u64;
                            }
                        }

                        if vc.height() > 1 {
                            if let Ok(sec_mode_val) = values.get(1) {
                                feature.sec_mode =
                                    sec_mode_val.to_string().trim_matches('"').to_string();
                                if let Ok(sec_mode_freq) =
                                    counts.get(1).unwrap().try_extract::<u64>()
                                {
                                    feature.sec_mode_frequency = sec_mode_freq;
                                    feature.sec_mode_percent =
                                        ((sec_mode_freq as f64 / total_rows) * 100.0) as u64;
                                }
                            }
                        }
                    }
                }
            }

            features.push(feature);
        }

        Ok((features, lazy_df))
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
