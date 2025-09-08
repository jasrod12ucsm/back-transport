use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::format,
    mem,
};

use ac_struct_back::{
    import::macro_import::TableName,
    schemas::config::{
        country::country::{Country, UpdateCountryError},
        country_timezone::country_timezone::CountryTimezone,
        timezone::timezone::Timezone,
    },
    utils::domain::{
        query::{
            BulkUpdateQueryBuilder, Condition, Expression, GraphBuilder, OneOrMany, Operator,
            Query, UpdateRequest, comparison, execute_select_query, execute_update_query,
        },
        relations::{RelateRequest, execute_relate_query},
        surreal::{Geometry, GeometryMultiPolygon, GeometryPoint, GeometryPolygon},
    },
};
use common::utils::ntex_private::extractors::{
    json::JsonAdvanced, multipart_extractor::MultipartData,
};
use ntex::http::StatusCode;
use serde_json::Value;
use surrealdb::{RecordId, Surreal, engine::any::Any};

use crate::{
    modules::country::domain::{
        data::{
            create_country_dto::CreateCountryDto, create_country_response::CreateCountryResponse,
            natural_earth_geometry_dto::NaturalEarthFeatureDto,
        },
        models::{
            country_id::CountryId, country_name::CountryName, country_to_update::CountryToUpdate,
        },
        use_case::create_country_use_case::{CreateCountryUseCase, CreateCountryUseCaseTrait},
    },
    try_get_surreal_pool,
};
use crate::{
    modules::country::domain::{
        data::{
            create_country_timezone_dto::{CountryTimezoneDto, CreateCountryTimezoneDto},
            natural_earth_geometry_dto::NaturalEarthGeometryDto,
        },
        models::{
            country_cca2::CountryCCA2, country_name::TimezoneCountryProyection,
            country_only_name::CountryOnlyName, country_timezone_to_insert::InsertCountryTimezone,
            timezone_id::TimezoneId,
        },
    },
    utils::charge_models::void_struct::VoidStruct,
};
use itertools::Itertools;
#[async_trait::async_trait]
impl CreateCountryUseCaseTrait for CreateCountryUseCase {
    async fn create_country(
        dto: MultipartData<VoidStruct>,
    ) -> Result<JsonAdvanced<CreateCountryResponse>, UpdateCountryError> {
        let db = try_get_surreal_pool()
            .ok_or_else(|| UpdateCountryError {
                message: format!("Could not get surreal pool"),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .get()
            .await
            .map_err(|_| UpdateCountryError {
                message: format!("Could not get surreal pool"),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        //procesar la data y extraer solamente el archivo json si?
        println!("paso0");
        let (data, country_timezone, earth_data) = Self::process_data(dto)?;
        let (countries_insert, countries_update, country_timezone) =
            Self::desition_of_insert_data(data, earth_data, country_timezone, &db.client).await?;
        println!("paso13");
        let update_countries = Self::update_countries(countries_update, &db.client).await?;
        println!("paso14");
        let insert_countries = Self::insert_countries(countries_insert, &db.client).await?;
        println!("paso15");
        let country_timezones =
            Self::create_country_timezones(country_timezone, &insert_countries, &db.client).await?;
        let val = CreateCountryResponse::new(update_countries, insert_countries, country_timezones);

        println!("paso16");
        Ok(JsonAdvanced(val))
    }
}

impl CreateCountryUseCase {
    //procesar la data y extraer solamente el archivo json si?
    async fn create_country_timezones(
        data: Vec<InsertCountryTimezone>,
        countries_inserted: &Vec<Country>,
        db: &Surreal<Any>,
    ) -> Result<Vec<CountryTimezone>, UpdateCountryError> {
        let mut timezones_names = HashSet::new();
        let countries_ids: HashMap<String, String> = countries_inserted
            .into_iter()
            .map(|x| (x.cca2.clone(), x.id.clone().unwrap().key().to_string()))
            .collect();
        for country in data.clone() {
            for timezone in country.timezones {
                let name = timezone.zone_name;
                timezones_names.insert(name);
            }
        }
        let timezones_ids: Vec<TimezoneId> = match execute_select_query(
            Query::<Timezone>::new()
                .from(None, false)
                .condition(Condition::Comparison {
                    left: Expression::Field(Cow::Borrowed("name")),
                    op: Operator::In,
                    right: Expression::Value(Value::Array(
                        timezones_names.into_iter().map(Value::String).collect(),
                    )),
                })
                .fields(&["id", "name"])
                .get_owned(),
            db,
            false,
        )
        .await
        .map_err(|_: UpdateCountryError| UpdateCountryError {
            message: "Error al obtener los timezones".to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })? {
            OneOrMany::One(_) => {
                return Err(UpdateCountryError {
                    message: "Error al obtener los timezones".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                });
            }
            OneOrMany::Many(val) => val,
        };
        //transform to HashMap
        let timezones_ids_hash: HashMap<String, TimezoneId> = timezones_ids
            .into_iter()
            .map(|x| (x.name.clone(), x))
            .collect();
        let mut vec_result = vec![];

        for country in data {
            let cca2_id = match country.id_key {
                Some(val) => val,
                None => {
                    let val =
                        countries_ids
                            .get(&country.iso2)
                            .ok_or_else(|| UpdateCountryError {
                                message: "Country not found".to_string(),
                                status_code: StatusCode::NOT_FOUND,
                            })?;
                    val.clone()
                }
            };
            let timezones = country.timezones;
            let timezones_ids: Vec<String> = timezones
                .into_iter()
                .map(|x| {
                    timezones_ids_hash
                        .get(&x.zone_name)
                        .unwrap()
                        .id
                        .clone()
                        .unwrap()
                        .key()
                        .to_string()
                })
                .collect();
            //hacer query para traer los country timezones
            if !timezones_ids.len() == 0 {
                let mut created: Vec<CountryTimezone> = execute_relate_query(
                    RelateRequest::<CountryTimezone>::builder()
                        .from(cca2_id.clone().as_str())
                        .to_vec(timezones_ids.iter().map(|a| a.as_str()).collect())
                        .content(CountryTimezone {
                            ..Default::default()
                        })
                        .map_err(|e| UpdateCountryError {
                            message: "Error al crear el timezone".to_string(),
                            status_code: StatusCode::INTERNAL_SERVER_ERROR,
                        })?
                        .get_owned(),
                    db,
                    false,
                )
                .await
                .map_err(|e: UpdateCountryError| UpdateCountryError {
                    message: "Error al crear el timezone".to_string() + &e.message,
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })?;
                vec_result.append(&mut created);
            }
        }
        Ok(vec_result)
    }
    async fn update_countries(
        data: Vec<CountryToUpdate>,
        db: &Surreal<Any>,
    ) -> Result<Vec<Country>, UpdateCountryError> {
        let countries: HashMap<String, Country> = Self::select_countries_by_cca2(
            data.clone()
                .into_iter()
                .map(|x| CountryOnlyName { cca2: x.cca2 })
                .collect(),
            db,
        )
        .await?
        .into_iter()
        .map(|x| (x.cca2.clone(), x))
        .collect();

        let data: Vec<CountryToUpdate> = data
            .into_iter()
            .filter(|x: &CountryToUpdate| {
                //compare with the country in the database
                let country = countries.get(&x.cca2);
                match country {
                    Some(val) => {
                        //compare all fields
                        let compare_multi_polygon = match &val.geo_multi_polygon {
                            Some(val) => match &x.geo_multi_polygon {
                                Some(val2) => {
                                    val.coordinates == val2.coordinates
                                        && val.type_geometry == val2.type_geometry
                                }
                                None => false,
                            },
                            None => {
                                //si multipoligon es none tmb devuelve true si no false
                                match &x.geo_multi_polygon {
                                    Some(_) => false,
                                    None => true,
                                }
                            }
                        };
                        let compare_polygon = match &val.geo_polygon {
                            Some(val) => match &x.geo_polygon {
                                Some(val2) => {
                                    val.coordinates == val2.coordinates
                                        && val.type_geometry == val2.type_geometry
                                }
                                None => false,
                            },
                            None => {
                                //si multipoligon es none tmb devuelve true si no false
                                match &x.geo_polygon {
                                    Some(_) => false,
                                    None => true,
                                }
                            }
                        };
                        !(val.cca2 == x.cca2
                            && val.cca3 == x.cca3
                            && val.ccn3 == x.ccn3
                            && val.flag == x.flag
                            && val.point.coordinates == x.latlng
                            && compare_multi_polygon
                            && compare_polygon)
                    }
                    None => false,
                }
            })
            .collect();
        if data.len() == 0 {
            return Ok(vec![]);
        }
        let chunk_size = 20;
        let mut countries_inserted = vec![];

        for chunk in data.chunks(chunk_size) {
            let batch = chunk.to_vec(); // ahora cada batch es un Vec<CountryToUpdate>

            let mut builder =
                BulkUpdateQueryBuilder::<Country, CountryToUpdate>::new(batch.clone());
            builder.target(|_| Country::table_name().to_string());
            builder.condition_dynamic(|a| Condition::Comparison {
                left: Expression::Field(Cow::Borrowed("cca2")),
                op: Operator::Eq,
                right: Expression::Field(Cow::Owned(format!("\"{}\"", a.cca2.clone()))),
            });
            builder.set_dynamic("cca2", |a| Value::String(a.cca2.clone()));
            builder.set_dynamic("cca3", |a| Value::String(a.cca3.clone()));
            builder.set_dynamic("ccn3", |a| Value::String(a.ccn3.clone()));
            builder.set_dynamic("flag", |a| Value::String(a.flag.clone()));
            builder.set_dynamic("geo_polygon", |a| match &a.geo_polygon {
                Some(val) => serde_json::to_value(val).unwrap(),
                None => Value::Null,
            });
            builder.set_dynamic("geo_multi_polygon", |a| match &a.geo_multi_polygon {
                Some(val) => serde_json::to_value(val).unwrap(),
                None => Value::Null,
            });
            builder.set_dynamic("point", |a| {
                serde_json::to_value(GeometryPoint {
                    type_geometry: "Point".to_string(),
                    coordinates: vec![a.latlng[0], a.latlng[1]],
                })
                .unwrap()
            });
            builder.set_strict(false);

            let mut build_query = builder.build().map_err(|e| UpdateCountryError {
                message: format!("Could not build query: {}", e),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
            let query = build_query
                .build_surreal_query()
                .map_err(|e| UpdateCountryError {
                    message: format!("Could not build surreal query: {}", e),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })?;
            println!("query: {:?}", query);
            let parameters = build_query.parameters().clone();
            println!("parameters: {:?}", parameters);

            let result =
                db.query(query)
                    .bind(parameters)
                    .await
                    .map_err(|e| UpdateCountryError {
                        message: format!("Could not execute query: {}", e),
                        status_code: StatusCode::INTERNAL_SERVER_ERROR,
                    })?;
            result.check().unwrap();

            let batch_inserted = Self::select_countries_by_cca2(
                batch
                    .into_iter()
                    .map(|x| CountryOnlyName { cca2: x.cca2 })
                    .collect(),
                db,
            )
            .await?;

            countries_inserted.extend(batch_inserted);
        }
        Ok(countries_inserted)
    }

    async fn select_countries_by_cca2(
        cca2: Vec<CountryOnlyName>,
        db: &Surreal<Any>,
    ) -> Result<Vec<Country>, UpdateCountryError> {
        let query = Query::<Country>::new()
            .from(None, false)
            .condition(Condition::Comparison {
                left: Expression::Field(Cow::Borrowed("cca2")),
                op: Operator::In,
                right: Expression::Value(cca2.into_iter().map(|x| x.cca2).collect()),
            })
            .get_owned();
        let str = query.to_surreal_query(false);
        println!("query: {:?}", str);
        let result: OneOrMany<Country> = execute_select_query(query, db, false).await?;
        println!("paso312312");
        match result {
            OneOrMany::One(_) => Err(UpdateCountryError {
                message: format!("Internal Error "),
                status_code: StatusCode::CONFLICT,
            }),
            OneOrMany::Many(val) => Ok(val),
        }
    }

    fn process_data(
        mut data: MultipartData<VoidStruct>,
    ) -> Result<
        (
            Vec<CreateCountryDto>,
            Vec<CreateCountryTimezoneDto>,
            NaturalEarthGeometryDto,
        ),
        UpdateCountryError,
    > {
        let preload_file = data.take_files().ok_or_else(|| UpdateCountryError {
            message: format!("Could not get files"),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })?;
        let mut country_file = None;
        let mut country_timezone_file = None;
        let mut natural_earth_file = None;
        for file in preload_file.into_iter() {
            println!("file name {:?}", file.file_name);
            if file.file_name == "countries.json" {
                country_file = Some(file);
            } else if file.file_name == "timezone_countries.json" {
                country_timezone_file = Some(file);
            } else if file.file_name == "countries_shapefile.json" {
                natural_earth_file = Some(file);
            } else {
                return Err(UpdateCountryError {
                    message: format!("Invalid file name"),
                    status_code: StatusCode::BAD_REQUEST,
                });
            }
        }
        //too be Some
        if country_file.is_none() || country_timezone_file.is_none() || natural_earth_file.is_none()
        {
            return Err(UpdateCountryError {
                message: format!("Invalid file name"),
                status_code: StatusCode::BAD_REQUEST,
            });
        }
        //parser bytes to vec of CreateCountryDto by serde array
        println!("paso1");
        let data: Vec<CreateCountryDto> = serde_json::from_slice(&country_file.unwrap().file_data)
            .map_err(|e| {
                println!("error en el parser {:?}", e);
                UpdateCountryError {
                    message: format!("Could not parse bytes to vec of CreateCountryDto"),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                }
            })?;
        println!("paso2");
        let country_timezone_data =
            serde_json::from_slice(&country_timezone_file.unwrap().file_data).map_err(|e| {
                UpdateCountryError {
                    message: format!("Could not parse bytes to vec of CreateCountryTimezoneDto"),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                }
            })?;
        let natural_earth_data = serde_json::from_slice(&natural_earth_file.unwrap().file_data)
            .map_err(|e| UpdateCountryError {
                message: format!("Could not parse bytes Geo {}", e),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        Ok((data, country_timezone_data, natural_earth_data))
    }
    //verificar si existe el nombre
    async fn verify_existance(
        db: &Surreal<Any>,
    ) -> Result<HashMap<String, (Vec<TimezoneCountryProyection>, RecordId)>, UpdateCountryError>
    {
        let graph_expression = GraphBuilder::new()
            .out(CountryTimezone::table_name())
            .out(Timezone::table_name())
            .project_object()
            .add_field("id", Expression::Field(Cow::Borrowed("id")))
            .add_field("name", Expression::Field(Cow::Borrowed("name")))
            .build()
            .build();
        //verificar si existe
        let exist: OneOrMany<CountryName> = execute_select_query(
            Query::<Country>::new()
                .from(None, false)
                .add_field(
                    Expression::Graph(Box::new(graph_expression)),
                    Some("timezone"),
                )
                //
                //ADD ID
                .add_field(Expression::Field(Cow::Borrowed("id")), None)
                .add_field(Expression::Field(Cow::Borrowed("cca2")), None)
                .get_owned(),
            db,
            false,
        )
        .await?;
        match exist {
            OneOrMany::One(_) => Err(UpdateCountryError {
                message: format!("Internal Error "),
                status_code: StatusCode::CONFLICT,
            }),
            OneOrMany::Many(val) => Ok(val
                .into_iter()
                .map(|x| (x.cca2, (x.timezone, x.id)))
                .collect()),
        }
    }
    async fn desition_of_insert_data(
        data: Vec<CreateCountryDto>,
        geometry_data: NaturalEarthGeometryDto,
        timezones_data: Vec<CreateCountryTimezoneDto>,
        db: &Surreal<Any>,
    ) -> Result<
        (
            Vec<Country>,
            Vec<CountryToUpdate>,
            Vec<InsertCountryTimezone>,
        ),
        UpdateCountryError,
    > {
        let mut existence_countries = Self::verify_existance(db).await?;
        let mut countries_insert = vec![];
        let mut countries_update = vec![];
        let mut geometry_country_hash_map: HashMap<String, NaturalEarthFeatureDto> = geometry_data
            .features
            .into_iter()
            .map(|x| (x.properties.cca2.clone(), x))
            .collect();

        let mut timezones: HashMap<String, Vec<CountryTimezoneDto>> = timezones_data
            .into_iter()
            .map(|x| (x.iso2, x.timezones))
            .collect();
        let mut result_relations_timezones = vec![];

        for country in data {
            //verifica si existe
            let existante = existence_countries.get_mut(&country.cca2);
            let existance_timezones = timezones.get_mut(&country.cca2);
            let existance_geometry = geometry_country_hash_map.get_mut(&country.cca2);

            if existante.is_none() {
                let timezones = if let Some(timezones) = existance_timezones {
                    let mut_vec_existance: &mut Vec<CountryTimezoneDto> = timezones.as_mut();
                    let existence = mem::take(mut_vec_existance);
                    existence
                } else {
                    vec![]
                };
                let mut geo_polygon = None;
                let mut geo_multi_polygon = None;
                if let Some(geometry) = existance_geometry {
                    match geometry.geometry.clone() {
                        Geometry::Polygon(val) => {
                            geo_polygon = Some(GeometryPolygon {
                                coordinates: val,
                                type_geometry: "Polygon".to_string(),
                            })
                        }
                        Geometry::MultiPolygon(val) => {
                            geo_multi_polygon = Some(GeometryMultiPolygon {
                                coordinates: val,
                                type_geometry: "MultiPolygon".to_string(),
                            })
                        }
                        _ => {}
                    }
                }
                let country = Country {
                    name: country.name.common,
                    cca2: country.cca2,
                    cca3: country.cca3,
                    ccn3: country.ccn3,
                    flag: country.flag,
                    point: ac_struct_back::utils::domain::surreal::GeometryPoint {
                        type_geometry: "Point".to_string(),
                        coordinates: vec![country.latlng[0], country.latlng[1]],
                    },
                    geo_polygon,
                    geo_multi_polygon,
                    ..Default::default()
                };

                result_relations_timezones.push(InsertCountryTimezone {
                    id_key: None,
                    iso2: country.cca2.clone(),
                    timezones,
                });

                //agregalo al vector de insersion
                countries_insert.push(country);
            } else {
                let mut_vec_existance = existante.unwrap();
                let existence_country: HashSet<String> = mut_vec_existance
                    .0
                    .iter()
                    .map(|x| x.name().to_string())
                    .collect();
                let id = mut_vec_existance.1.key().to_string();
                // compara el existance con los timezones del dataset, y si no existe, lo agrega,
                // si existe lo quita
                let timezones = if let Some(timezones) = existance_timezones {
                    let mut_vec_existance: &mut Vec<CountryTimezoneDto> = timezones.as_mut();
                    let existence = std::mem::take(mut_vec_existance);
                    let value_to_insert: Vec<CountryTimezoneDto> = existence
                        .into_iter()
                        .filter(|x| !existence_country.contains(&x.zone_name.to_string()))
                        .collect();
                    value_to_insert
                } else {
                    vec![]
                };
                let mut geo_polygon = None;
                let mut geo_multi_polygon = None;
                if let Some(geometry) = existance_geometry {
                    match geometry.geometry.clone() {
                        Geometry::Polygon(val) => {
                            geo_polygon = Some(GeometryPolygon {
                                coordinates: val,
                                type_geometry: "Polygon".to_string(),
                            })
                        }
                        Geometry::MultiPolygon(val) => {
                            geo_multi_polygon = Some(GeometryMultiPolygon {
                                coordinates: val,
                                type_geometry: "MultiPolygon".to_string(),
                            })
                        }
                        _ => {}
                    }
                }
                result_relations_timezones.push(InsertCountryTimezone {
                    id_key: Some(id),
                    iso2: country.cca2.clone(),
                    timezones,
                });
                let country_to_update = CountryToUpdate {
                    name: country.name.clone(),
                    cca2: country.cca2.clone(),
                    cca3: country.cca3.clone(),
                    ccn3: country.ccn3.clone(),
                    latlng: country.latlng.clone(),
                    flag: country.flag.clone(),
                    geo_polygon: geo_polygon,
                    geo_multi_polygon: geo_multi_polygon,
                };
                drop(country);

                countries_update.push(country_to_update);
            }
        }
        Ok((
            countries_insert,
            countries_update,
            result_relations_timezones,
        ))
    }
    async fn insert_countries(
        countries: Vec<Country>,
        db: &Surreal<Any>,
    ) -> Result<Vec<Country>, UpdateCountryError> {
        let insert_result: Vec<Country> = db
            .insert(Country::table_name())
            .content(countries)
            .await
            .map_err(|e| UpdateCountryError {
                message: format!("Failed to insert countries: {}", e),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        Ok(insert_result)
    }
}
