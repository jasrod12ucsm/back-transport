use ac_struct_back::{
    schemas::config::template_type::template_type::{DeleteTemplateTypeError, TemplateType},
    utils::domain::query::UpdateRequest,
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::http::StatusCode;

use crate::{
    modules::template_type::domain::use_cases::delete_template_type_use_case::{
        DeleteTemplateTypeUseCase, DeleteTemplateTypeUseCasePublic,
    },
    try_get_surreal_pool,
};

#[async_trait::async_trait]
impl DeleteTemplateTypeUseCasePublic for DeleteTemplateTypeUseCase {
    async fn execute(
        template_type: &str,
    ) -> Result<JsonAdvanced<TemplateType>, DeleteTemplateTypeError> {
        let pool = try_get_surreal_pool()
            .ok_or_else(|| DeleteTemplateTypeError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to get SurrealDB pool".to_string(),
            })?
            .get()
            .await
            .map_err(|_| DeleteTemplateTypeError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to get SurrealDB connection".to_string(),
            })?;
        let conn = &pool.client;

        //construir query
        let query = UpdateRequest::<TemplateType>::builder()
            .new_soft_delete(template_type)
            .map_err(|_| DeleteTemplateTypeError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to delete Template".to_string(),
            })?
            .build()
            .map_err(|_| DeleteTemplateTypeError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to deleteTemplate".to_string(),
            })?;
        let query_str = query
            .build_surreal_query(true)
            .map_err(|_| DeleteTemplateTypeError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to parse request".to_string(),
            })?;
        //ejecutar query
        let parameters = query.parameters;

        let deleted_template: Vec<TemplateType> = conn
            .query(query_str)
            .bind(parameters)
            .await
            .map_err(|_| DeleteTemplateTypeError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to execute delete query".to_string(),
            })?
            .take(0)
            .map_err(|_| DeleteTemplateTypeError {
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
                message: "Failed to retrieve deleted template".to_string(),
            })?;
        if deleted_template.is_empty() {
            Err(DeleteTemplateTypeError {
                status_code: StatusCode::NOT_FOUND,
                message: "Template type not found".to_string(),
            })
        } else {
            Ok(JsonAdvanced(deleted_template.into_iter().next().unwrap()))
        }
    }
}
