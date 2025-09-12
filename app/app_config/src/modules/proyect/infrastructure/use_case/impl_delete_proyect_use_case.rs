use ac_struct_back::{
    schemas::config::{
        proyect::proyect::{DeleteProjectError, Proyect},
        template::template::Template,
    },
    utils::domain::query::UpdateRequest,
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

use crate::{
    modules::proyect::domain::use_case::delete_proyect_use_case::{
        DeleteProyectUseCase, DeleteProyectUseCasePublic,
    },
    try_get_surreal_pool,
};

#[async_trait::async_trait]
impl DeleteProyectUseCasePublic for DeleteProyectUseCase {
    async fn execute(template_type: &str) -> Result<JsonAdvanced<Proyect>, DeleteProjectError> {
        let pool = try_get_surreal_pool()
            .ok_or_else(|| DeleteProjectError::FatalError)?
            .get()
            .await
            .map_err(|_| DeleteProjectError::FatalError)?;
        let conn = &pool.client;

        //construir query
        let query = UpdateRequest::<Proyect>::builder()
            .new_soft_delete(template_type)
            .map_err(|_| DeleteProjectError::FatalError)?
            .build()
            .map_err(|_| {
                DeleteProjectError::DbError(
                    "Error al validar la consulta de eliminación".to_string(),
                )
            })?;
        let query_str = query
            .build_surreal_query(false)
            .map_err(|_| DeleteProjectError::DbError("Error al parsear datos".to_string()))?;

        //ejecutar query
        let parameters = query.parameters;
        println!("query_str: {:?}", query_str);

        let deleted_proyect: Vec<Proyect> = conn
            .query(query_str)
            .bind(parameters)
            .await
            .map_err(|_| DeleteProjectError::FatalError)?
            .take(0)
            .map_err(|_| DeleteProjectError::FatalError)?;
        if deleted_proyect.is_empty() {
            Err(DeleteProjectError::NotFoundError)
        } else {
            Ok(JsonAdvanced(deleted_proyect.into_iter().next().unwrap()))
        }
    }
}
