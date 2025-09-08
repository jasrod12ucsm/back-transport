use std::{borrow::Cow, collections::HashSet};

use ac_struct_back::{
    schemas::config::timezone::timezone::{CreateTimezoneError, Timezone},
    utils::domain::query::{
        Condition, Expression, OneOrMany, Operator, Query, execute_select_query,
    },
};
use common::utils::ntex_private::extractors::multipart_extractor::MultipartData;
use ntex::http::StatusCode;
use serde_json::Value;
use surrealdb::{Surreal, engine::any::Any};

use crate::{
    modules::timezone::domain::{
        data::create_timezone_dto::CreateTimezoneDto,
        models::timezone_name::TimezoneName,
        repository::timezone_repository::TimezoneRepository,
        use_case::create_timezone_use_case::{CreateTimezoneUseCase, CreateTimezoneUseCaseTrait},
    },
    try_get_surreal_pool,
    utils::charge_models::void_struct::VoidStruct,
};
#[async_trait::async_trait]
impl CreateTimezoneUseCaseTrait for CreateTimezoneUseCase {
    async fn execute(dto: MultipartData<VoidStruct>) -> Result<Vec<Timezone>, CreateTimezoneError> {
        let db = try_get_surreal_pool()
            .ok_or_else(|| CreateTimezoneError {
                message: "No se encontró el pool de surreal".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .get()
            .await
            .map_err(|_| CreateTimezoneError {
                message: "Error al obtener el pool de surreal".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        let dto = Self::process_multipart_data(dto)?;
        let timezones = Self::flatten_timezones(dto, &db.client).await?;
        Self::insert_timezones(timezones, &db.client).await
    }
}

impl CreateTimezoneUseCase {
    //primero se procesa el multipart_data y se extrae el archivo del dto
    fn process_multipart_data(
        mut dto: MultipartData<VoidStruct>,
    ) -> Result<Vec<CreateTimezoneDto>, CreateTimezoneError> {
        let file = dto
            .take_files()
            .ok_or_else(|| CreateTimezoneError {
                message: "No se encontró el archivo".to_string(),
                status_code: StatusCode::BAD_REQUEST,
            })?
            .into_iter()
            .find(|f| f.file_name == "timezone_countries.json")
            .ok_or_else(|| CreateTimezoneError {
                message: "No se encontró el archivo con el nombre esperado".to_string(),
                status_code: StatusCode::BAD_REQUEST,
            })?;
        //serde para deserializar el json
        let data: Vec<CreateTimezoneDto> =
            serde_json::from_slice(&file.file_data).map_err(|e| {
                println!("{:?}", e);
                CreateTimezoneError {
                    message: format!("Error al deserializar el archivo"),
                    status_code: StatusCode::BAD_REQUEST,
                }
            })?;
        Ok(data)
    }
    async fn flatten_timezones(
        dto: Vec<CreateTimezoneDto>,
        db: &Surreal<Any>,
    ) -> Result<Vec<Timezone>, CreateTimezoneError> {
        let mut seen = HashSet::new();
        let mut candidates = Vec::new();

        for country in dto {
            for tz in country.timezones {
                if seen.insert(tz.zone_name.clone()) {
                    candidates.push(Timezone {
                        id: None,
                        name: tz.zone_name,
                        gmt_offset: tz.gmt_offset,
                        gmt_offset_name: tz.gmt_offset_name,
                        ..Default::default()
                    });
                }
            }
        }

        let existing_names = Self::get_existing_timezone_names(
            db,
            &candidates.iter().map(|tz| &tz.name).collect::<HashSet<_>>(),
        )
        .await?;

        let result = candidates
            .into_iter()
            .filter(|tz| !existing_names.contains(&tz.name))
            .collect();

        Ok(result)
    }

    async fn get_existing_timezone_names(
        db: &Surreal<Any>,
        names: &HashSet<&String>,
    ) -> Result<HashSet<String>, CreateTimezoneError> {
        let values: Vec<Value> = names
            .iter()
            .map(|name| Value::from((*name).clone()))
            .collect();

        let result: OneOrMany<TimezoneName> = execute_select_query(
            Query::<Timezone>::new()
                .from(None, false)
                .condition(Condition::Comparison {
                    left: Expression::Field(Cow::Borrowed("name")),
                    op: Operator::In,
                    right: Expression::Value(Value::from("$name")),
                })
                .fields(&["name"])
                .parameter("name", Value::Array(values))
                .get_owned(),
            db,
            true,
        )
        .await
        .map_err(|e: CreateTimezoneError| {
            println!("{:?}", e);
            CreateTimezoneError {
                message: "Error al consultar nombres existentes".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

        let set = match result {
            OneOrMany::One(Some(tz)) => HashSet::from([tz.name]),
            OneOrMany::One(None) => HashSet::new(),
            OneOrMany::Many(vec) => vec.into_iter().map(|tz| tz.name).collect(),
        };

        Ok(set)
    }

    async fn insert_timezones(
        timezones: Vec<Timezone>,
        db: &Surreal<Any>,
    ) -> Result<Vec<Timezone>, CreateTimezoneError> {
        Timezone::insert_timezones(timezones, db)
            .await
            .map_err(|_: surrealdb::Error| CreateTimezoneError {
                message: "Error al insertar los timezones".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })
    }
}
