use ac_struct_back::{
    schemas::config::template::template::{DeleteTemplateError, Template},
    utils::domain::query::UpdateRequest,
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::http::StatusCode;

use crate::{
    modules::template::domain::use_case::delete_template_use_case::{
        DeleteTemplateUseCase, DeleteTemplateUseCasePublic,
    },
    try_get_surreal_pool,
};

#[async_trait::async_trait]
impl DeleteTemplateUseCasePublic for DeleteTemplateUseCase {
    async fn execute(template_type: &str) -> Result<JsonAdvanced<Template>, DeleteTemplateError> {
        let pool = try_get_surreal_pool()
            .ok_or_else(|| DeleteTemplateError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Error de Servidor Interno".to_string(),
            })?
            .get()
            .await
            .map_err(|_| DeleteTemplateError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Error de Servidor Interno".to_string(),
            })?;
        let conn = &pool.client;

        //construir query
        let query = UpdateRequest::<Template>::builder()
            .new_soft_delete(template_type)
            .map_err(|_| DeleteTemplateError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Error al eliminar Template".to_string(),
            })?
            .build()
            .map_err(|_| DeleteTemplateError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Error al Eliminar Template".to_string(),
            })?;
        let query_str = query
            .build_surreal_query(false)
            .map_err(|_| DeleteTemplateError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Error al Parsear Datos".to_string(),
            })?;
        //ejecutar query
        let parameters = query.parameters;

        let deleted_template: Vec<Template> = conn
            .query(query_str)
            .bind(parameters)
            .await
            .map_err(|_| DeleteTemplateError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Error en el Proceso".to_string(),
            })?
            .take(0)
            .map_err(|_| DeleteTemplateError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Error al Eliminar ".to_string(),
            })?;
        if deleted_template.is_empty() {
            Err(DeleteTemplateError {
                status_code: StatusCode::NOT_FOUND,
                message: "Template No Encontrado".to_string(),
            })
        } else {
            Ok(JsonAdvanced(deleted_template.into_iter().next().unwrap()))
        }
    }
}
