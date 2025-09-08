use ac_struct_back::{
    import::macro_import::TableName,
    schemas::config::template_type::template_type::{
        createtemplatetypedtotemplatetype::CreateTemplateTypeDto, TemplateType, TemplateTypeError,
    },
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::http::StatusCode;
use surrealdb::opt::Resource;

use crate::{
    modules::template_type::domain::use_cases::create_template_type_use_case::{
        CreateTemplateTypeUseCase, CreateTemplateTypeUseCasePublic,
    },
    try_get_surreal_pool,
};

#[async_trait::async_trait]
impl CreateTemplateTypeUseCasePublic for CreateTemplateTypeUseCase {
    async fn execute(
        &self,
        dto: CreateTemplateTypeDto,
    ) -> Result<JsonAdvanced<TemplateType>, TemplateTypeError> {
        let db_conection = try_get_surreal_pool()
            .ok_or_else(|| TemplateTypeError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .get()
            .await
            .map_err(|_| TemplateTypeError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        let CreateTemplateTypeDto { name, description } = dto;

        //verifica si el nombre ya existe
        let tmp = TemplateType {
            name: name,
            description: description,
            ..Default::default()
        };
        //crear el template
        let conn = &db_conection.client;
        let template_type: Option<TemplateType> = conn
            .create(TemplateType::table_name())
            .content(tmp)
            .await
            .map_err(|_| TemplateTypeError {
                message: "Error al crear el tipo de plantilla".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        if let Some(template_type) = template_type {
            Ok(JsonAdvanced(template_type))
        } else {
            Err(TemplateTypeError {
                message: "Error al crear el tipo de plantilla".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })
        }
    }
}
