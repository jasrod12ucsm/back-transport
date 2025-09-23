use std::{mem, sync::Arc};

use common::utils::ntex_private::extractors::{
    json::JsonAdvanced, multipart_extractor::MultipartData,
};
use polars::{
    io::SerReader,
    prelude::{CsvEncoding, CsvParseOptions, CsvReadOptions, CsvReader, PlSmallStr},
};

use crate::{
    modules::app_charge::domain::{
        response::get_columns_response::GetColumnsResponse,
        use_case::get_columns_use_case::{GetColumnsUseCase, GetColumnsUseCaseTrait},
    },
    utils::{charge_models::void_struct::VoidStruct, errors::csv_error::CsvError},
};

#[async_trait::async_trait]
impl GetColumnsUseCaseTrait for GetColumnsUseCase {
    async fn charge_file(
        dto: MultipartData<VoidStruct>,
    ) -> Result<JsonAdvanced<GetColumnsResponse>, CsvError> {
        println!("ahora proceso");
        let data = Self::process_files(dto).await?;
        let response = GetColumnsResponse {
            fields: data.iter().map(|x| x.to_string()).collect(),
        };
        Ok(JsonAdvanced(response))
    }
}

impl GetColumnsUseCase {
    async fn process_files(
        mut data: MultipartData<VoidStruct>,
    ) -> Result<Vec<PlSmallStr>, CsvError> {
        let body = data
            .get_data()
            .ok_or_else(|| CsvError::FileChargeError)?
            .clone();
        let separator = body.separator.unwrap_or(",".to_string());
        let preload_file = data.take_files().ok_or_else(|| CsvError::FileChargeError)?;
        if preload_file.is_empty() || preload_file.len() > 1 {
            return Err(CsvError::FileChargeError);
        }
        //es csv validator
        println!("passo1");
        if preload_file.get(0).unwrap().extension != "csv" {
            return Err(CsvError::InvalidFileType);
        }
        let mut files = preload_file; // asumiendo que ya es tuyo
        let file = files.remove(0);
        let bytes = file.file_data; // ahora sí tienes ownership
        let cursor = std::io::Cursor::new((&bytes).as_ref());
        let parse_opts = CsvParseOptions {
            separator: separator.as_bytes()[0], // lo más común: coma como separador
            quote_char: Some(b'"'),             // campos entre comillas dobles
            eol_char: b'\n',                    // salto de línea estándar
            encoding: CsvEncoding::Utf8,        // hoy en día casi todo es UTF-8
            null_values: None,                  // NULLs detectados por defecto (vacíos)
            missing_is_null: true,              // celdas vacías = null
            truncate_ragged_lines: true,        // si faltan columnas, completa con nulls
            comment_prefix: None,               // sin soporte de comentarios
            try_parse_dates: true,              // intenta detectar fechas
            decimal_comma: false,               // por defecto: decimal con punto (.)
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
        let collumns = df.get_column_names_owned();
        Ok(collumns)
    }
}
