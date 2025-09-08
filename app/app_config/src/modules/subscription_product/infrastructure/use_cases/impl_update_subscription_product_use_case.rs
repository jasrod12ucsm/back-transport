use ac_struct_back::{
    schemas::auth::subscription_product::subscription_product::{
        SubscriptionProduct, UpdateSubscriptionProductError,
        updatesubscriptionproductdtosubscriptionproduct::UpdateSubscriptionProductDto,
    },
    utils::domain::{
        front_query::{UpdateRequestBuilderFront, UpdateRequestFront},
        query::{Query, UpdateRequest, UpdateRequestBuilder, UpdateTarget},
    },
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::http::StatusCode;
use serde_json::Value;

use crate::modules::subscription_product::domain::use_case::update_subscription_product_use_case::{UpdateSubscriptionProductUseCase, UpdateSubscriptionProductUseCaseTrait};
#[async_trait::async_trait]
impl UpdateSubscriptionProductUseCaseTrait for UpdateSubscriptionProductUseCase {
    async fn execute(
        id: Option<String>,
        request: UpdateRequestBuilderFront<SubscriptionProduct>,
        dto: JsonAdvanced<UpdateSubscriptionProductDto>,
    ) -> Result<JsonAdvanced<Vec<SubscriptionProduct>>, UpdateSubscriptionProductError> {
        let conn = crate::try_get_surreal_pool()
            .ok_or_else(|| UpdateSubscriptionProductError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .get()
            .await
            .map_err(|_| UpdateSubscriptionProductError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        let mut model = request.flat().map_err(|e| {
            println!("{:?}", e);
            UpdateSubscriptionProductError {
                message: "Error al construir la consulta de actualización".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;
        if id.is_none() && model.conditions.is_empty() {
            return Err(UpdateSubscriptionProductError {
                message: "Error al actualizar la plantilla".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
        let builder = UpdateRequestBuilder::<SubscriptionProduct>::new();
        let mut query = Query::<SubscriptionProduct>::new()
            .from(id.as_deref(), false)
            .get_owned();
        model
            .prepare()
            .map_err(|_| UpdateSubscriptionProductError {
                message: "Error al preparar la solicitud de actualización".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;

        query.select.conditions = model.conditions;
        let UpdateSubscriptionProductDto {
            name,
            description,
            status,
        } = dto.0;
        let mut q = builder
            .update(Some(UpdateTarget::Subquery(query)))
            .map_err(|e| {
                println!("{:?}", e);
                UpdateSubscriptionProductError {
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
                serde_json::to_value(status).map_err(|_| UpdateSubscriptionProductError {
                    message: "Error al construir la consulta de actualización".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })?,
            );
        }
        let mut q_buld_query = q.build().map_err(|e| {
            println!("{:?}", e);
            UpdateSubscriptionProductError {
                message: "Error al construir la consulta de actualización".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            }
        })?;

        q_buld_query.parameters.extend(model.parameters);

        let query_str =
            q_buld_query
                .build_surreal_query(true)
                .map_err(|_| UpdateSubscriptionProductError {
                    message: "Error al construir la consulta de actualización".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })?;
        let parameters = q_buld_query.parameters;

        let updated_subscription_product: Vec<SubscriptionProduct> = conn
            .client
            .query(query_str)
            .bind(parameters)
            .await
            .map_err(|_| UpdateSubscriptionProductError {
                message: "Error al actualizar la plantilla".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .take(0)
            .map_err(|_| UpdateSubscriptionProductError {
                message: "Error al actualizar la plantilla".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        if id.is_none() {
            Ok(JsonAdvanced(updated_subscription_product))
        } else {
            if updated_subscription_product.len() == 0 {
                Err(UpdateSubscriptionProductError {
                    message: "Error al actualizar la plantilla".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })
            } else {
                Ok(JsonAdvanced(updated_subscription_product))
            }
        }
    }
}
