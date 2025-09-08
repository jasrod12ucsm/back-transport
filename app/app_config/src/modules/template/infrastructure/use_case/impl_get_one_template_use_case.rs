use std::borrow::Cow;

use ac_struct_back::{
    schemas::config::template::template::{GetTemplatesError, Template},
    utils::domain::{
        front_query::QueryFront,
        query::{OneOrMany, Query},
    },
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::http::StatusCode;
use serde_json::Value;

use crate::{
    modules::template::domain::use_case::get_one_template_use_case::{
        GetOneTemplateUseCase, GetOneTemplateUseCasePublic,
    },
    try_get_surreal_pool,
};

#[async_trait::async_trait]
impl GetOneTemplateUseCasePublic for GetOneTemplateUseCase {
    async fn execute(
        query: QueryFront<Template>,
        id: &str,
    ) -> Result<JsonAdvanced<Option<Template>>, GetTemplatesError> {
        //get connection
        let conn = try_get_surreal_pool()
            .ok_or_else(|| GetTemplatesError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .get()
            .await
            .map_err(|_| GetTemplatesError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        //*construit query traer todo de template types */
        let mut model: Query<Template> = query.into();
        let redefined_query = model
            .from(Some(id), true)
            .condition(
                ac_struct_back::utils::domain::query::Condition::Comparison {
                    left: ac_struct_back::utils::domain::query::Expression::Field(Cow::Borrowed(
                        "deleted_at",
                    )),
                    op: ac_struct_back::utils::domain::query::Operator::Eq,
                    right: ac_struct_back::utils::domain::query::Expression::Value(Value::from(
                        "$val",
                    )),
                },
            )
            .parameter("val", Value::from(None::<String>))
            .get_owned();
        let template: OneOrMany<Template> =
            ac_struct_back::utils::domain::query::execute_select_query(
                redefined_query,
                &conn.client,
                true,
            )
            .await
            .map_err(|_: GetTemplatesError| GetTemplatesError {
                message: "Error al obtener la data".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        match template {
            OneOrMany::One(template) => {
                return Ok(JsonAdvanced(template));
            }
            OneOrMany::Many(_) => {
                return Err(GetTemplatesError {
                    message: "Error al obtener la data".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                });
            }
        }
    }
}
