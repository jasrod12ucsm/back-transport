use ac_struct_back::{
    schemas::config::template::template::{
        Template, UpdateTemplateError, updatetemplatedtotemplate::UpdateTemplateDto,
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
    modules::template::domain::use_case::update_template_use_case::{
        UpdateTemplateUseCase, UpdateTemplateUseCasePublic,
    },
    try_get_surreal_pool,
};

#[async_trait::async_trait]
impl UpdateTemplateUseCasePublic for UpdateTemplateUseCase {
    async fn execute(
        id: Option<String>,
        request: UpdateRequestBuilderFront<Template>,
        dto: UpdateTemplateDto,
    ) -> Result<JsonAdvanced<Vec<Template>>, UpdateTemplateError> {
        //* traier conesion */
        let conn = try_get_surreal_pool()
            .ok_or_else(|| UpdateTemplateError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .get()
            .await
            .map_err(|_| UpdateTemplateError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        //verifica que la tabla enviada si hay subquery sea la correcta
        let mut model = request.flat().map_err(|e| {
            println!("{:?}", e);
            UpdateTemplateError {
                message: "Error al construir la consulta de actualización".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;
        if id.is_none() && model.conditions.is_empty() {
            return Err(UpdateTemplateError {
                message: "Error al actualizar el tipo de plantilla".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
        let builder = UpdateRequestBuilder::<Template>::new();
        let mut query = Query::<Template>::new()
            .from(id.as_deref(), false)
            .get_owned();
        query.deleted_at();
        model.prepare().map_err(|_| UpdateTemplateError {
            message: "Error al preparar la solicitud de actualización".to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })?;

        query.select.conditions = model.conditions;
        let UpdateTemplateDto {
            name,
            description,
            status,
            type_id,
            template_str,
            required_fields,
        } = dto;

        let mut q = builder
            .update(Some(UpdateTarget::Subquery(query)))
            .map_err(|e| {
                println!("{:?}", e);
                UpdateTemplateError {
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
                serde_json::to_value(status).map_err(|_| UpdateTemplateError {
                    message: "Error al construir la consulta de actualización".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })?,
            );
        }
        if let Some(type_id) = type_id {
            q_ref.set("type_id", "t");
            q_ref.parameter(
                "t",
                serde_json::to_value(type_id).map_err(|_| UpdateTemplateError {
                    message: "Error al construir la consulta de actualización".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })?,
            );
        }
        if let Some(template_str) = template_str {
            q_ref.set("template_str", "ts");
            q_ref.parameter("ts", Value::from(template_str));
        }
        if let Some(required_fields) = required_fields {
            q_ref.set("required_fields", "rf");
            q_ref.parameter("rf", Value::from(required_fields));
        }
        q_ref.add_deleted_at_param();
        let mut q_build_query = q.build().map_err(|e| {
            println!("{:?}", e);
            UpdateTemplateError {
                message: "Error al construir la consulta de actualización".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;
        q_build_query.parameters.extend(model.parameters);
        let query_str =
            q_build_query
                .build_surreal_query(true)
                .map_err(|_| UpdateTemplateError {
                    message: "Error al construir la consulta de actualización".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })?;
        let parameters = q_build_query.parameters;

        let updated_template: Vec<Template> = conn
            .client
            .query(query_str)
            .bind(parameters)
            .await
            .map_err(|e| UpdateTemplateError {
                message: "Error al actualizar el tipo de plantilla".to_string()
                    + ":"
                    + &e.to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .take(0)
            .map_err(|e| UpdateTemplateError {
                message: "Error al actualizar el tipo de plantilla".to_string()
                    + ":"
                    + &e.to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        if id.is_none() {
            Ok(JsonAdvanced(updated_template))
        } else {
            if updated_template.len() == 0 {
                Err(UpdateTemplateError {
                    message: "Error al actualizar el tipo de plantilla no actualizado".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })
            } else {
                Ok(JsonAdvanced(updated_template))
            }
        }
    }
}
