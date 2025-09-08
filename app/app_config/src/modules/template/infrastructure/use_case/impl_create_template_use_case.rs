use ac_struct_back::{
    import::macro_import::TableName,
    schemas::config::{
        template::template::{
            CreateTemplateError, Template, createtemplatedtotemplate::CreateTemplateDto,
        },
        template_type::template_type::{TemplateType, templatetypeidtemplatetype::TemplateTypeId},
    },
    utils::domain::query::{OneOrMany, Query, execute_select_query},
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::http::StatusCode;

use crate::{
    modules::template::domain::use_case::create_template_use_case::{
        CreateTemplateUseCase, CreateTemplateUseCasePublic,
    },
    try_get_surreal_pool,
};

#[async_trait::async_trait]
impl CreateTemplateUseCasePublic for CreateTemplateUseCase {
    async fn execute(
        &self,
        dto: CreateTemplateDto,
    ) -> Result<JsonAdvanced<Template>, CreateTemplateError> {
        let db_conection = try_get_surreal_pool()
            .ok_or_else(|| CreateTemplateError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?
            .get()
            .await
            .map_err(|_| CreateTemplateError {
                message: "Error de Servidor Interno".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        let CreateTemplateDto {
            name,
            description,
            required_fields,
            type_id,
            template_str,
        } = dto;
        //verifica si la tabla de tipos de plantillas coincide con el id
        //el template str tiene dentro campos {URL} O {D1} Y YO tengo required fields que debe
        //coincidir con esos campos, encuentralos todos los required fields dentro del template
        //dentro de {} y si no estan devuelve Error
        let required_fields_with_bracktes = required_fields
            .iter()
            .map(|x| format!("{{{}}}", x))
            .collect::<Vec<String>>();
        fn all_fields_in_template_aho(
            template_str: &str,
            required_fields: &[String],
        ) -> Result<(), CreateTemplateError> {
            //usa aho_corasick para buscar todos los required fields dentro del template
            let aho = aho_corasick::AhoCorasick::new(required_fields).map_err(|_| {
                CreateTemplateError {
                    message: "Error al crear el tipo de plantilla".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                }
            })?;
            for field in required_fields {
                if !aho.is_match(template_str) {
                    return Err(CreateTemplateError {
                        message: "Error al crear el tipo de plantilla".to_string(),
                        status_code: StatusCode::INTERNAL_SERVER_ERROR,
                    });
                }
            }
            Ok(())
        }
        //usa la funcion
        all_fields_in_template_aho(&template_str, &required_fields_with_bracktes)?;

        if TemplateType::table_name().to_string() != type_id.table() {
            return Err(CreateTemplateError {
                message: "Error al crear el tipo de plantilla".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
        //validate tempalte_type exist in db
        let template_type: OneOrMany<TemplateTypeId> = execute_select_query(
            Query::<TemplateType>::new()
                .from(Some(type_id.key().to_string().as_str()), true)
                .fields(&["id"])
                .get_owned(),
            &db_conection.client,
            true,
        )
        .await?;
        match template_type {
            OneOrMany::One(template_type) => {
                if template_type.is_none() {
                    return Err(CreateTemplateError {
                        message: "Error al crear el tipo de plantilla".to_string(),
                        status_code: StatusCode::INTERNAL_SERVER_ERROR,
                    });
                }
            }
            OneOrMany::Many(_) => {
                return Err(CreateTemplateError {
                    message: "Error al crear el tipo de plantilla".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                });
            }
        }

        //verifica si el nombre ya existe
        let tmp = Template {
            name: name,
            description: description,
            required_fields: required_fields,
            type_id: type_id,
            template_str: template_str,
            ..Default::default()
        };
        //crear el template
        let conn = &db_conection.client;
        let template_type: Option<Template> = conn
            .create(Template::table_name())
            .content(tmp)
            .await
            .map_err(|_| CreateTemplateError {
                message: "Error al crear el tipo de plantilla".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
        if let Some(template_type) = template_type {
            Ok(JsonAdvanced(template_type))
        } else {
            Err(CreateTemplateError {
                message: "Error al crear el tipo de plantilla".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })
        }
    }
}
