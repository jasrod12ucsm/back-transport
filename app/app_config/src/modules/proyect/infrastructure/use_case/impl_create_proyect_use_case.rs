use std::borrow::Cow;

use ac_struct_back::{
    import::macro_import::TableName,
    schemas::config::proyect::proyect::{CreateProjectError, Proyect},
    utils::domain::query::{Condition, OneOrMany, Operator, Query, execute_select_query},
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use serde_json::Value;

use crate::{
    modules::proyect::domain::{
        data::create_proyect_dto::CreateProyectDto,
        use_case::create_proyect_use_case::{CreateProyectUseCase, CreateProyectUseCasePublic},
    },
    try_get_surreal_pool,
};

#[async_trait::async_trait]
impl CreateProyectUseCasePublic for CreateProyectUseCase {
    async fn execute(
        &self,
        dto: CreateProyectDto,
    ) -> Result<JsonAdvanced<Proyect>, CreateProjectError> {
        let db_conection = try_get_surreal_pool()
            .ok_or_else(|| CreateProjectError::FatalError)?
            .get()
            .await
            .map_err(|_| CreateProjectError::FatalError)?;
        let CreateProyectDto { name } = dto;
        //verifica si la tabla de tipos de plantillas coincide con el id
        //el template str tiene dentro campos {URL} O {D1} Y YO tengo required fields que debe
        //coincidir con esos campos, encuentralos todos los required fields dentro del template
        //dentro de {} y si no estan devuelve Error
        //validate tempalte_type exist in db
        //verifica si el nombre ya existe
        let tmp = Proyect {
            name: name.clone(),
            ..Default::default()
        };
        //sonultar si ya existe por el nombre
        let proyect: OneOrMany<Proyect> = execute_select_query(
            Query::<Proyect>::new()
                .from(None, true)
                .condition(Condition::Comparison {
                    left: ac_struct_back::utils::domain::query::Expression::Field(Cow::Borrowed(
                        "name",
                    )),
                    op: ac_struct_back::utils::domain::query::Operator::Eq,
                    right: ac_struct_back::utils::domain::query::Expression::Value("$name".into()),
                })
                .parameter("name", name.into())
                .limit("1")
                .get_owned(),
            &db_conection.client,
            true,
        )
        .await?;

        match proyect {
            OneOrMany::One(proyect) => {
                if proyect.is_some() {
                    return Err(CreateProjectError::AlreadyExistsError);
                }
            }
            OneOrMany::Many(_) => {
                return Err(CreateProjectError::DbError(
                    "Error en la consulta".to_string(),
                ));
            }
        };
        //crear el template
        let conn = &db_conection.client;
        let proyect: Option<Proyect> = conn
            .create(Proyect::table_name())
            .content(tmp)
            .await
            .map_err(|_| CreateProjectError::DbError("Error al crear la db".to_string()))?;
        if let Some(proyect) = proyect {
            Ok(JsonAdvanced(proyect))
        } else {
            Err(CreateProjectError::DbError(
                "Error al crear la db".to_string(),
            ))
        }
    }
}
