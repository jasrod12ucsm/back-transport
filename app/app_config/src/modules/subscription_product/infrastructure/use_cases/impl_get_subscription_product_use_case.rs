use std::borrow::Cow;

use ac_struct_back::{
    schemas::auth::subscription_product::subscription_product::{
        GetSubscriptionProductError, SubscriptionProduct,
    },
    utils::domain::{
        front_query::QueryFront,
        query::{Condition, Expression, OneOrMany, Query, execute_select_query},
    },
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::http::StatusCode;
use serde_json::Value;

use crate::modules::subscription_product::domain::use_case::get_subscription_product_use_case::{
    GetSubscriptionProductUseCase, GetSubscriptionProductUseCaseTrait,
};

#[async_trait::async_trait]
impl GetSubscriptionProductUseCaseTrait for GetSubscriptionProductUseCase {
    async fn execute(
        query: QueryFront<SubscriptionProduct>,
    ) -> Result<JsonAdvanced<Vec<SubscriptionProduct>>, GetSubscriptionProductError> {
        let conn = crate::try_get_surreal_pool()
            .ok_or_else(|| GetSubscriptionProductError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .get()
            .await
            .map_err(|_| GetSubscriptionProductError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        let mut model: Query<SubscriptionProduct> = query.into();
        model.condition(Condition::Comparison {
            left: Expression::Field(Cow::Borrowed("deleted_at")),
            op: ac_struct_back::utils::domain::query::Operator::Eq,
            right: Expression::Value(Value::from("$del")),
        });
        model.parameter("del", Value::from(None::<String>));
        //agregar a la query el ide porque solo tiene condiciones
        let redefined_query = model.from(None, false).get_owned();
        let subscription: OneOrMany<SubscriptionProduct> =
            execute_select_query(redefined_query, &conn.client, true)
                .await
                .map_err(|e: GetSubscriptionProductError| {
                    println!("{:?}", e);
                    GetSubscriptionProductError {
                        message: "Error al obtener la data".to_string(),
                        status_code: StatusCode::INTERNAL_SERVER_ERROR,
                    }
                })?;
        match subscription {
            OneOrMany::One(_) => {
                return Err(GetSubscriptionProductError {
                    message: "Error al obtener la data".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                });
            }
            OneOrMany::Many(val) => {
                return Ok(JsonAdvanced(val));
            }
        }
    }
}
