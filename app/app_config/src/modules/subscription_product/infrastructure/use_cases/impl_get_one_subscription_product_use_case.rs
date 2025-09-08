use std::borrow::Cow;

use ac_struct_back::{
    schemas::auth::subscription_product::subscription_product::{
        GetOneSubscriptionProductError, SubscriptionProduct,
    },
    utils::domain::{
        front_query::QueryFront,
        query::{Condition, Expression, OneOrMany, Query, execute_select_query},
    },
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::http::StatusCode;
use serde_json::Value;

use crate::modules::subscription_product::domain::use_case::get_one_subscription_prodcut_use_case::{GetOneSubscriptionProductUseCase, GetOneSubscriptionProductUseCaseTrait};

#[async_trait::async_trait]
impl GetOneSubscriptionProductUseCaseTrait for GetOneSubscriptionProductUseCase {
    async fn execute(
        query: QueryFront<SubscriptionProduct>,
        id: &str,
    ) -> Result<JsonAdvanced<Option<SubscriptionProduct>>, GetOneSubscriptionProductError> {
        let conn = crate::try_get_surreal_pool()
            .ok_or_else(|| GetOneSubscriptionProductError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .get()
            .await
            .map_err(|_| GetOneSubscriptionProductError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        let mut model: Query<SubscriptionProduct> = query.into();
        //agregar a la query el ide porque solo tiene condiciones
        let redefined_query = model
            .from(Some(id), true)
            .condition(Condition::Comparison {
                left: Expression::Field(Cow::Borrowed("deleted_at")),
                op: ac_struct_back::utils::domain::query::Operator::Eq,
                right: Expression::Value(Value::from("$val")),
            })
            .parameter("val", Value::from(None::<String>))
            .get_owned();
        let subscription: OneOrMany<SubscriptionProduct> =
            execute_select_query(redefined_query, &conn.client, true)
                .await
                .map_err(
                    |_: GetOneSubscriptionProductError| GetOneSubscriptionProductError {
                        message: "Error al obtener la data".to_string(),
                        status_code: StatusCode::INTERNAL_SERVER_ERROR,
                    },
                )?;
        match subscription {
            OneOrMany::One(subscription) => {
                return Ok(JsonAdvanced(subscription));
            }
            OneOrMany::Many(_) => {
                return Err(GetOneSubscriptionProductError {
                    message: "Error al obtener la data".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                });
            }
        }
    }
}
