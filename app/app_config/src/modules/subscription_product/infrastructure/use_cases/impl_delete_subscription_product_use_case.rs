use ac_struct_back::{
    schemas::auth::subscription_product::subscription_product::{
        DeleteSubscriptionProductError, SubscriptionProduct,
    },
    utils::domain::query::{Query, UpdateRequest},
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::http::StatusCode;

use crate::modules::subscription_product::domain::use_case::delete_subscription_product_use_case::{DeleteSubscriptionProductUseCase, DeleteSubscriptionProductUseCasePublic};

#[async_trait::async_trait]
impl DeleteSubscriptionProductUseCasePublic for DeleteSubscriptionProductUseCase {
    async fn execute(
        id: &str,
    ) -> Result<JsonAdvanced<SubscriptionProduct>, DeleteSubscriptionProductError> {
        let conn = crate::try_get_surreal_pool()
            .ok_or_else(|| DeleteSubscriptionProductError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .get()
            .await
            .map_err(|_| DeleteSubscriptionProductError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        let model = UpdateRequest::<SubscriptionProduct>::builder()
            .new_soft_delete(id)
            .map_err(|_| DeleteSubscriptionProductError {
                message: "Error al eliminar la data".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .build()
            .map_err(|_| DeleteSubscriptionProductError {
                message: "Error al eliminar la data".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        let query_str =
            model
                .build_surreal_query(true)
                .map_err(|_| DeleteSubscriptionProductError {
                    message: "Error al construir la consulta de eliminación de datos".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })?;
        let parameters = model.parameters;

        let delete_subscription_p: Vec<SubscriptionProduct> = conn
            .client
            .query(query_str)
            .bind(parameters)
            .await
            .map_err(|_| DeleteSubscriptionProductError {
                message: "Error al eliminar la data".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .take(0)
            .map_err(|_| DeleteSubscriptionProductError {
                message: "Error al eliminar la data".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        if delete_subscription_p.is_empty() {
            Err(DeleteSubscriptionProductError {
                message: "Error al obtener la data".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })
        } else {
            Ok(JsonAdvanced(
                delete_subscription_p.into_iter().next().unwrap(),
            ))
        }
    }
}
