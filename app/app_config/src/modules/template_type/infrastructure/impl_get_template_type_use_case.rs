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
    modules::template_type::domain::use_cases::get_templates_type_use_case::{
        GetTemplatesTypeUseCase, GetTemplatesTypeUseCasePublic,
    },
    try_get_surreal_pool,
};

#[async_trait::async_trait]
impl GetTemplatesTypeUseCasePublic for GetTemplatesTypeUseCase {
    async fn execute(
        query: QueryFront<TemplateType>,
    ) -> Result<JsonAdvanced<Vec<TemplateType>>, TemplateTypesNotFoundError> {
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

        model.condition(Condition::Comparison {
            left: ac_struct_back::utils::domain::query::Expression::Field(Cow::Borrowed(
                "deleted_at",
            )),
            op: Operator::Eq,
            right: ac_struct_back::utils::domain::query::Expression::Value(Value::from("$del")),
        });
        model.parameter("del", Value::from(None::<String>));
        let redefined_query = model.from(None, false).get_owned();
        //* ejecutar la consulta */
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
            OneOrMany::One(_) => {
                return Err(TemplateTypesNotFoundError {
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
