use std::borrow::Cow;

use ac_struct_back::{
    schemas::config::proyect::proyect::{GetOneProjectError, Proyect},
    utils::domain::{
        front_query::QueryFront,
        query::{Condition, OneOrMany, Operator, Query},
    },
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use serde_json::Value;

use crate::{
    modules::proyect::domain::use_case::get_one_proyect_use_case::{
        GetOneProyectUseCase, GetOneProyectUseCasePublic,
    },
    try_get_surreal_pool,
};

#[async_trait::async_trait]
impl GetOneProyectUseCasePublic for GetOneProyectUseCase {
    async fn execute(
        query: QueryFront<Proyect>,
        id: &str,
    ) -> Result<JsonAdvanced<Option<Proyect>>, GetOneProjectError> {
        //get connection
        let conn = try_get_surreal_pool()
            .ok_or_else(|| GetOneProjectError::FatalError)?
            .get()
            .await
            .map_err(|_| GetOneProjectError::FatalError)?;
        //*construit query traer todo de template types */
        let mut model: Query<Proyect> = query.into();
        let redefined_query = model
            .from(Some(id), true)
            .condition(Condition::Comparison {
                left: ac_struct_back::utils::domain::query::Expression::Field(Cow::Borrowed(
                    "deleted_at",
                )),
                op: Operator::Eq,
                right: ac_struct_back::utils::domain::query::Expression::Value(Value::from("$val")),
            })
            .parameter("val", Value::from(None::<String>))
            .get_owned();
        let proyect: OneOrMany<Proyect> =
            ac_struct_back::utils::domain::query::execute_select_query(
                redefined_query,
                &conn.client,
                true,
            )
            .await
            .map_err(|_: GetOneProjectError| GetOneProjectError::FatalError)?;
        match proyect {
            OneOrMany::One(template_type) => {
                //if is none return error
                if template_type.is_none() {
                    return Err(GetOneProjectError::NotFoundError);
                }
                return Ok(JsonAdvanced(template_type));
            }
            OneOrMany::Many(_) => {
                return Err(GetOneProjectError::DbError(
                    "Error executing query".to_string(),
                ));
            }
        }
    }
}
