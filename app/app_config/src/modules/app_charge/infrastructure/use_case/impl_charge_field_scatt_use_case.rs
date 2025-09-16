use std::{borrow::Cow, sync::Arc};

use ac_struct_back::{
    import::macro_import::TableName,
    schemas::config::{
        feature::{feature::Feature, feature_type::FeatureType},
        proyect::proyect::Proyect,
        proyect_feature::proyect_feature::ProyectFeature,
    },
    utils::domain::query::{GraphBuilder, OneOrMany, Query, execute_select_query},
};
use common::utils::ntex_private::extractors::{
    json::JsonAdvanced, multipart_extractor::MultipartData,
};
use polars::{
    frame::DataFrame,
    io::SerReader,
    prelude::{CsvEncoding, CsvParseOptions, CsvReadOptions, CsvReader, IntoLazy},
};
use serde_json::{Value, json};

use crate::{
    modules::app_charge::{
        domain::{
            models::proyect_desnormalized::ProyectDesnormalized,
            response::file_charge_response::FileChargeResponse,
            use_case::charge_file_use_case::ChargeFileUseCase,
        },
        infrastructure::use_case::impl_get_all_data_use_case::DATA_FRAMES,
    },
    try_get_surreal_pool,
    utils::{charge_models::void_struct::VoidStruct, errors::csv_error::CsvError},
};

pub struct ChargeFieldScatterUseCase;
#[async_trait::async_trait]
pub trait ChargeFieldScatterUseCaseTrait {
    async fn charge_file(
        dto: MultipartData<VoidStruct>,
        proyect_id: String,
        field_id: String,
    ) -> Result<JsonAdvanced<FileChargeResponse>, CsvError>;
}

#[async_trait::async_trait]
impl ChargeFieldScatterUseCaseTrait for ChargeFieldScatterUseCase {
    async fn charge_file(
        dto: MultipartData<VoidStruct>,
        proyect_id: String,
        field_id: String,
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
            &conn,
            false,
        )
        .await?;

        let proyect: ProyectDesnormalized = match exist {
            OneOrMany::One(_) => Err(CsvError::FileChargeError),
            OneOrMany::Many(val) => {
                if val.len() > 1 || val.is_empty() {
                    Err(CsvError::FileChargeError)
                } else {
                    let first = val.into_iter().next().unwrap();
                    Ok(first)
                }
            }
        }?;

        let params_in = json!({ "in" : format!("mst_feature:{}", field_id) });

        let query = "SELECT in from mst_feature_to_feature where in= <record>$in";

        let mut in_response = (&conn).query(query).bind(params_in).await.map_err(|e| {
            println!("{:?}", e);
            CsvError::FileChargeError
        })?;

        let values_in: Vec<Value> = in_response.take(0).map_err(|e| {
            println!("{:?}", e);
            CsvError::FileChargeError
        })?;
        if values_in.len() > 1 {
            return Err(CsvError::FileChargeError);
        }
        //now find the feature
        let feature: Vec<Feature> = proyect
            .clone()
            .fields
            .into_iter()
            .filter(|f| matches!(f.type_feature, FeatureType::Continuous))
            .collect();
        let feature_only = proyect
            .fields
            .into_iter()
            .find(|f| f.id.as_ref().unwrap().key().to_string() == field_id)
            .ok_or(CsvError::FileChargeError)?;
        println!(
            "feature_only id {}",
            feature_only.id.as_ref().unwrap().key().to_string()
        );

        //get first proyect
        println!("llego a apasa el proyect");

        let data = Self::process_files(dto, proyect_id.clone()).await?;
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
        keep_only_key(proyect_id.as_str(), data.clone());
        println!("llego a procesar los datos");
        ChargeFileUseCase::process_scatterplots(
            feature.iter().collect(),
            vec![&feature_only],
            data.lazy(),
            conn,
        )
        .await?;

        Ok(JsonAdvanced(FileChargeResponse { ok: true }))
    }
}

impl ChargeFieldScatterUseCase {
    async fn process_files(
        mut data: MultipartData<VoidStruct>,
        proyect_id: String,
    ) -> Result<DataFrame, CsvError> {
        let preload_file = data.take_files().ok_or_else(|| CsvError::FileChargeError)?;
        if preload_file.len() > 1 {
            return Err(CsvError::FileChargeError);
        }

        // Bloque principal de mutación del mapa
        {
            let mut map = DATA_FRAMES.write().unwrap();

            // Si no existe el DataFrame para este proyecto
            if !map.contains_key(proyect_id.as_str()) {
                if let Some(file) = preload_file.into_iter().next() {
                    if file.extension != "csv" {
                        return Err(CsvError::InvalidFileType);
                    }

                    let bytes = file.file_data; // ownership del archivo
                    let cursor = std::io::Cursor::new(bytes);

                    let parse_opts = CsvParseOptions {
                        separator: b';',
                        quote_char: Some(b'"'),
                        eol_char: b'\n',
                        encoding: CsvEncoding::Utf8,
                        null_values: None,
                        missing_is_null: true,
                        truncate_ragged_lines: true,
                        comment_prefix: None,
                        try_parse_dates: true,
                        decimal_comma: false,
                    };

                    let options = CsvReadOptions {
                        has_header: true,
                        rechunk: true,
                        n_threads: None,
                        low_memory: false,
                        chunk_size: 1_000_000,
                        infer_schema_length: Some(1000),
                        ignore_errors: false,
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

                    // Insertar y mantener solo esta key
                    map.entry(proyect_id.to_string()).or_insert(df);
                    map.retain(|k, _| k.to_string() == proyect_id);
                } else {
                    return Err(CsvError::FileChargeError);
                }
            }
        } // aquí termina el bloque de escritura, liberando el RwLockWriteGuard

        // Ahora sí podemos leer de forma segura
        let map = DATA_FRAMES.read().unwrap();
        let dataframe = map
            .get(proyect_id.as_str())
            .ok_or(CsvError::FileChargeError)?;
        Ok(dataframe.clone())
    }
}
