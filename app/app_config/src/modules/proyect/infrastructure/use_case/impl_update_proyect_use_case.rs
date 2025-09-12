use ac_struct_back::{
    schemas::config::{
        proyect::proyect::{Proyect, UpdateProjectError},
        template::template::{
            Template, UpdateTemplateError, updatetemplatedtotemplate::UpdateTemplateDto,
        },
    },
    utils::domain::{
        front_query::UpdateRequestBuilderFront,
        query::{Query, UpdateRequestBuilder, UpdateTarget},
    },
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::http::StatusCode;
use serde_json::Value;

use crate::{
    modules::proyect::domain::{
        data::update_proyect_dto::UpdateProyectDto,
        use_case::update_proyect_use_case::{UpdateProyectUseCase, UpdateProyectUseCasePublic},
    },
    try_get_surreal_pool,
};

#[async_trait::async_trait]
impl UpdateProyectUseCasePublic for UpdateProyectUseCase {
    async fn execute(
        id: Option<String>,
        request: UpdateRequestBuilderFront<Proyect>,
        dto: UpdateProyectDto,
    ) -> Result<JsonAdvanced<Vec<Proyect>>, UpdateProjectError> {
        //* traier conesion */
        let conn = try_get_surreal_pool()
            .ok_or_else(|| UpdateProjectError::FatalError)?
            .get()
            .await
            .map_err(|_| UpdateProjectError::FatalError)?;
        //verifica que la tabla enviada si hay subquery sea la correcta
        let mut model = request.flat().map_err(|e| {
            println!("{:?}", e);
            UpdateProjectError::FatalError
        })?;
        if id.is_none() && model.conditions.is_empty() {
            return Err(UpdateProjectError::FatalError);
        }
        let builder = UpdateRequestBuilder::<Proyect>::new();
        let mut query = Query::<Proyect>::new()
            .from(id.as_deref(), false)
            .get_owned();
        query.deleted_at();
        model
            .prepare()
            .map_err(|_| UpdateProjectError::FatalError)?;

        query.select.conditions = model.conditions;
        let UpdateProyectDto { name } = dto;

        let mut q = builder
            .update(Some(UpdateTarget::Subquery(query)))
            .map_err(|e| {
                println!("{:?}", e);
                UpdateProjectError::DbError(
                    "Error al construir la consulta de actualización".to_string(),
                )
            })?;
        let q_ref = &mut q;

        q_ref.set("name", "n");
        q_ref.parameter("n", Value::from(name));
        q_ref.add_deleted_at_param();
        let mut q_build_query = q.build().map_err(|e| {
            println!("{:?}", e);
            UpdateProjectError::DbError("Error al validar la consulta de actualización".to_string())
        })?;
        q_build_query.parameters.extend(model.parameters);
        let query_str = q_build_query.build_surreal_query(true).map_err(|_| {
            UpdateProjectError::DbError(
                "Error al construir la consulta de actualización".to_string(),
            )
        })?;

        let parameters = q_build_query.parameters;

        let updated_template: Vec<Proyect> = conn
            .client
            .query(query_str)
            .bind(parameters)
            .await
            .map_err(|e| UpdateProjectError::FatalError)?
            .take(0)
            .map_err(|e| UpdateProjectError::FatalError)?;
        if id.is_none() {
            Ok(JsonAdvanced(updated_template))
        } else {
            if updated_template.len() == 0 {
                Err(UpdateProjectError::NotFoundError)
            } else {
                Ok(JsonAdvanced(updated_template))
            }
        }
    }
}
