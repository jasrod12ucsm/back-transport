use ac_struct_back::{
    import::macro_import::TableName,
    schemas::config::template_type::template_type::{
        TemplateType, UpdateTemplateTypeError,
        updatetemplatetypedtotemplatetype::UpdateTemplateTypeDto,
    },
    utils::domain::{
        front_query::{UpdateRequestBuilderFront, UpdateRequestFront},
        query::{Query, UpdateRequest, UpdateRequestBuilder, UpdateTarget},
    },
};
use chrono::Utc;
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::http::StatusCode;
use serde_json::Value;

use crate::{
    modules::template_type::domain::use_cases::update_request_use_case::{
        UpdateTemplateTypeUseCase, UpdateTemplateTypeUseCasePublic,
    },
    try_get_surreal_pool,
};

#[async_trait::async_trait]
impl UpdateTemplateTypeUseCasePublic for UpdateTemplateTypeUseCase {
    async fn execute(
        id: Option<String>,
        request: UpdateRequestBuilderFront<TemplateType>,
        dto: UpdateTemplateTypeDto,
    ) -> Result<JsonAdvanced<Vec<TemplateType>>, UpdateTemplateTypeError> {
        //* traier conesion */
        let conn = try_get_surreal_pool()
            .ok_or_else(|| UpdateTemplateTypeError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .get()
            .await
            .map_err(|_| UpdateTemplateTypeError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        //verifica que la tabla enviada si hay subquery sea la correcta
        let mut model = request.flat().map_err(|e| {
            println!("{:?}", e);
            UpdateTemplateTypeError {
                message: "Error al construir la consulta de actualización".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;
        if id.is_none() && model.conditions.is_empty() {
            return Err(UpdateTemplateTypeError {
                message: "Error al actualizar el tipo de plantilla".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
        let builder = UpdateRequestBuilder::<TemplateType>::new();
        let mut query = Query::<TemplateType>::new()
            .from(id.as_deref(), false)
            .get_owned();
        model.prepare().map_err(|_| UpdateTemplateTypeError {
            message: "Error al preparar la solicitud de actualización".to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })?;

        query.select.conditions = model.conditions;
        query.deleted_at();
        let UpdateTemplateTypeDto {
            name,
            description,
            status,
        } = dto;
        let mut q = builder
            .update(Some(UpdateTarget::Subquery(query)))
            .map_err(|e| {
                println!("{:?}", e);
                UpdateTemplateTypeError {
                    message: "Error al construir la consulta de actualización".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                }
            })?;
        let q_ref = &mut q;

        if let Some(name) = name {
            q_ref.set("name", "n");
            q_ref.parameter("n", Value::from(name));
        }
        if let Some(description) = description {
            q_ref.set("description", "d");
            q_ref.parameter("d", Value::from(description));
        }
        if let Some(status) = status {
            q_ref.set("status", "s");
            q_ref.parameter(
                "s",
                serde_json::to_value(status).map_err(|_| UpdateTemplateTypeError {
                    message: "Error al construir la consulta de actualización".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })?,
            );
        }
        q_ref.add_deleted_at_param();
        let mut q_build_query = q.build().map_err(|e| {
            println!("{:?}", e);
            UpdateTemplateTypeError {
                message: "Error al construir la consulta de actualización".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;
        q_build_query.parameters.extend(model.parameters);
        let query_str =
            q_build_query
                .build_surreal_query(true)
                .map_err(|_| UpdateTemplateTypeError {
                    message: "Error al construir la consulta de actualización".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })?;
        let parameters = q_build_query.parameters;

        let updated_template_type: Vec<TemplateType> = conn
            .client
            .query(query_str)
            .bind(parameters)
            .await
            .map_err(|_| UpdateTemplateTypeError {
                message: "Error al actualizar el tipo de plantilla".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .take(0)
            .map_err(|_| UpdateTemplateTypeError {
                message: "Error al actualizar el tipo de plantilla".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        if id.is_none() {
            Ok(JsonAdvanced(updated_template_type))
        } else {
            if updated_template_type.len() == 0 {
                Err(UpdateTemplateTypeError {
                    message: "Error al actualizar el tipo de plantilla".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })
            } else {
                Ok(JsonAdvanced(updated_template_type))
            }
        }
    }
}
