use ac_struct_back::{
    schemas::config::{
        country::country::{Country, UpdateCountryError},
        template::template::Template,
    },
    utils::domain::{
        front_query::UpdateRequestBuilderFront,
        query::{Query, UpdateRequestBuilder, UpdateTarget},
    },
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::http::StatusCode;

use crate::{
    modules::country::domain::{
        data::update_country_dto::UpdateCountryDto,
        use_case::update_country_use_case::{UpdateCountryUseCase, UpdateCountryUseCaseTrait},
    },
    try_get_surreal_pool,
};

#[async_trait::async_trait]
impl UpdateCountryUseCaseTrait for UpdateCountryUseCase {
    async fn execute(
        id: Option<String>,
        request: UpdateRequestBuilderFront<Country>,
        dto: UpdateCountryDto,
    ) -> Result<JsonAdvanced<Vec<Country>>, UpdateCountryError> {
        //* traier conesion */
        let conn = try_get_surreal_pool()
            .ok_or_else(|| UpdateCountryError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .get()
            .await
            .map_err(|_| UpdateCountryError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        //verifica que la tabla enviada si hay subquery sea la correcta
        let mut model = request.flat().map_err(|e| {
            println!("{:?}", e);
            UpdateCountryError {
                message: "Error al construir la consulta de actualización".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;
        if id.is_none() && model.conditions.is_empty() {
            return Err(UpdateCountryError {
                message: "Error al actualizar el tipo de plantilla".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
        let builder = UpdateRequestBuilder::<Template>::new();
        let mut query = Query::<Template>::new()
            .from(id.as_deref(), false)
            .get_owned();
        query.deleted_at();
        model.prepare().map_err(|_| UpdateCountryError {
            message: "Error al preparar la solicitud de actualización".to_string(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        })?;

        query.select.conditions = model.conditions;
        let UpdateCountryDto { status } = dto;

        let mut q = builder
            .update(Some(UpdateTarget::Subquery(query)))
            .map_err(|e| {
                println!("{:?}", e);
                UpdateCountryError {
                    message: "Error al construir la consulta de actualización".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                }
            })?;
        let q_ref = &mut q;

        if let Some(status) = status {
            q_ref.set("status", "s");
            q_ref.parameter(
                "s",
                serde_json::to_value(status).map_err(|_| UpdateCountryError {
                    message: "Error al construir la consulta de actualización".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })?,
            );
        }
        q_ref.add_deleted_at_param();
        let mut q_build_query = q.build().map_err(|e| {
            println!("{:?}", e);
            UpdateCountryError {
                message: "Error al construir la consulta de actualización".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;
        q_build_query.parameters.extend(model.parameters);
        let query_str =
            q_build_query
                .build_surreal_query(true)
                .map_err(|_| UpdateCountryError {
                    message: "Error al construir la consulta de actualización".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })?;
        let parameters = q_build_query.parameters;

        let updated_template: Vec<Country> = conn
            .client
            .query(query_str)
            .bind(parameters)
            .await
            .map_err(|_| UpdateCountryError {
                message: "Error al actualizar el tipo de plantilla".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .take(0)
            .map_err(|_| UpdateCountryError {
                message: "Error al actualizar el tipo de plantilla".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        if id.is_none() {
            Ok(JsonAdvanced(updated_template))
        } else {
            if updated_template.len() == 0 {
                Err(UpdateCountryError {
                    message: "Error al actualizar el tipo de plantilla".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })
            } else {
                Ok(JsonAdvanced(updated_template))
            }
        }
    }
}
