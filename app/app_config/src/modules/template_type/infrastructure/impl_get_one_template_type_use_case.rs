use std::borrow::Cow;

use ac_struct_back::{
    schemas::config::template_type::template_type::{TemplateType, TemplateTypesNotFoundError},
    utils::domain::{
        front_query::QueryFront,
        query::{Condition, OneOrMany, Operator, Query},
    },
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::http::StatusCode;
use serde_json::Value;

use crate::{
    modules::template_type::domain::use_cases::get_one_template_type_use_case::{
        GetOneTemplateTypeUseCase, GetOneTemplateTypeUseCasePublic,
    },
    try_get_surreal_pool,
};

#[async_trait::async_trait]
impl GetOneTemplateTypeUseCasePublic for GetOneTemplateTypeUseCase {
    async fn execute(
        query: QueryFront<TemplateType>,
        id: &str,
    ) -> Result<JsonAdvanced<Option<TemplateType>>, TemplateTypesNotFoundError> {
        //get connection
        let conn = try_get_surreal_pool()
            .ok_or_else(|| TemplateTypesNotFoundError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .get()
            .await
            .map_err(|_| TemplateTypesNotFoundError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        //*construit query traer todo de template types */
        let mut model: Query<TemplateType> = query.into();
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
        let template_types: OneOrMany<TemplateType> =
            ac_struct_back::utils::domain::query::execute_select_query(
                redefined_query,
                &conn.client,
                true,
            )
            .await
            .map_err(|_: TemplateTypesNotFoundError| TemplateTypesNotFoundError {
                message: "Error al obtener la data".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        match template_types {
            OneOrMany::One(template_type) => {
                return Ok(JsonAdvanced(template_type));
            }
            OneOrMany::Many(_) => {
                return Err(TemplateTypesNotFoundError {
                    message: "Error al obtener la data".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                });
            }
        }
    }
}
