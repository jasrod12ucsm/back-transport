use std::borrow::Cow;

use ac_struct_back::{
    schemas::config::country::country::{Country, GetCountryError, GetOneCountryError},
    utils::domain::{
        front_query::QueryFront,
        query::{Expression, OneOrMany, Query},
    },
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::http::StatusCode;
use serde_json::Value;

use crate::{
    modules::country::domain::use_case::get_one_country_use_case::{
        GetOneCountryUseCase, GetOneCountryeUseCaseTrait,
    },
    try_get_surreal_pool,
};

#[async_trait::async_trait]
impl GetOneCountryeUseCaseTrait for GetOneCountryUseCase {
    async fn execute(
        query: QueryFront<Country>,
        id: &str,
    ) -> Result<JsonAdvanced<Option<Country>>, GetOneCountryError> {
        //get connection
        let conn = try_get_surreal_pool()
            .ok_or_else(|| GetOneCountryError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .get()
            .await
            .map_err(|_| GetOneCountryError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        //*construit query traer todo de template types */
        let mut model: Query<Country> = query.into();
        let redefined_query = model
            .from(Some(id), true)
            .condition(
                ac_struct_back::utils::domain::query::Condition::Comparison {
                    left: Expression::Field(Cow::Borrowed("deleted_at")),
                    op: ac_struct_back::utils::domain::query::Operator::Eq,
                    right: Expression::Value(Value::from("$val")),
                },
            )
            .parameter("val", Value::from(None::<String>))
            .get_owned();
        let country: OneOrMany<Country> =
            ac_struct_back::utils::domain::query::execute_select_query(
                redefined_query,
                &conn.client,
                true,
            )
            .await
            .map_err(|e: GetCountryError| GetOneCountryError {
                message: "Error al obtener la data".to_string() + &e.message,
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        match country {
            OneOrMany::One(country) => {
                return Ok(JsonAdvanced(country));
            }
            OneOrMany::Many(_) => {
                return Err(GetOneCountryError {
                    message: "Error al obtener la data".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                });
            }
        }
    }
}
