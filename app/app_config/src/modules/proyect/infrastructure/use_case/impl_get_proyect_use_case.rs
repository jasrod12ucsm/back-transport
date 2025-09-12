use std::borrow::Cow;

use ac_struct_back::{
    schemas::config::{
        proyect::proyect::{GetProjectError, Proyect},
        template::template::{GetTemplatesError, Template},
    },
    utils::domain::{
        front_query::QueryFront,
        query::{Condition, OneOrMany, Query},
    },
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::http::StatusCode;
use serde_json::Value;

use crate::{
    modules::proyect::domain::use_case::get_proyect_use_case::{
        GetProyectsUseCase, GetProyectsUseCasePublic,
    },
    try_get_surreal_pool,
};

#[async_trait::async_trait]
impl GetProyectsUseCasePublic for GetProyectsUseCase {
    async fn execute(
        query: QueryFront<Proyect>,
    ) -> Result<JsonAdvanced<Vec<Proyect>>, GetProjectError> {
        //get connection
        let conn = try_get_surreal_pool()
            .ok_or_else(|| GetProjectError::FatalError)?
            .get()
            .await
            .map_err(|_| GetProjectError::FatalError)?;
        //*construit query traer todo de template types */
        let mut model: Query<Proyect> = query.into();
        model.condition(Condition::Comparison {
            left: ac_struct_back::utils::domain::query::Expression::Field(Cow::Borrowed(
                "deleted_at",
            )),
            op: ac_struct_back::utils::domain::query::Operator::Eq,
            right: ac_struct_back::utils::domain::query::Expression::Value(Value::from("$val")),
        });
        model.parameter("val", Value::from(None::<String>));
        let redefined_query = model.from(None, false).get_owned();
        //* ejecutar la consulta */
        let template: OneOrMany<Proyect> =
            ac_struct_back::utils::domain::query::execute_select_query(
                redefined_query,
                &conn.client,
                true,
            )
            .await
            .map_err(|_: GetProjectError| {
                GetProjectError::DbError("Error executing query".to_string())
            })?;
        match template {
            OneOrMany::One(_) => {
                return Err(GetProjectError::DbError(
                    "Error executing query complex".to_string(),
                ));
            }
            OneOrMany::Many(val) => {
                return Ok(JsonAdvanced(val));
            }
        }
    }
}
