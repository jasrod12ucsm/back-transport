use ac_struct_back::{
    schemas::config::template::template::{
        CreateTemplateError, DeleteTemplateError, GetTemplatesError, Template, UpdateTemplateError,
        createtemplatedtotemplate::CreateTemplateDto, updatetemplatedtotemplate::UpdateTemplateDto,
    },
    utils::domain::{
        front_query::{QueryFront, UpdateRequestBuilderFront},
        query::{Query, UpdateRequest},
    },
};
use common::utils::ntex_private::extractors::{json::JsonAdvanced, query_advanced::QueryAdvanced};
use ntex::{
    http::StatusCode,
    web::{self, types::Path},
};

use crate::modules::template::domain::use_case::{
    create_template_use_case::{CreateTemplateUseCase, CreateTemplateUseCasePublic},
    delete_template_use_case::{DeleteTemplateUseCase, DeleteTemplateUseCasePublic},
    get_one_template_use_case::{GetOneTemplateUseCase, GetOneTemplateUseCasePublic},
    get_template_use_case::{GetTemplatesUseCase, GetTemplatesUseCasePublic},
    update_template_use_case::{UpdateTemplateUseCase, UpdateTemplateUseCasePublic},
};

#[web::get("/")]
async fn get_templates(
    query: QueryAdvanced<QueryFront<Template>>,
) -> Result<JsonAdvanced<Vec<Template>>, GetTemplatesError> {
    GetTemplatesUseCase::execute(query.0).await
}

#[web::post("/")]
async fn create_template(
    dto: JsonAdvanced<CreateTemplateDto>,
) -> Result<JsonAdvanced<Template>, CreateTemplateError> {
    let case = CreateTemplateUseCase {};
    case.execute(dto.0).await
}
#[web::patch("/")]
async fn update_template(
    dto: JsonAdvanced<UpdateTemplateDto>,
    query: QueryAdvanced<UpdateRequestBuilderFront<Template>>,
) -> Result<JsonAdvanced<Vec<Template>>, UpdateTemplateError> {
    UpdateTemplateUseCase::execute(None, query.0, dto.0).await
}

#[web::patch("/{id}")]
async fn update_template_by_id(
    path: Path<String>,
    dto: JsonAdvanced<UpdateTemplateDto>,
    query: QueryAdvanced<UpdateRequestBuilderFront<Template>>,
) -> Result<JsonAdvanced<Template>, UpdateTemplateError> {
    let id = path.into_inner();
    UpdateTemplateUseCase::execute(Some(id), query.0, dto.0)
        .await
        .and_then(|wrapped_vec| {
            // Suponiendo que wrapped_vec es JsonAdvanced<Vec<Template>>
            // y queremos obtener el primer Template, si existe
            match wrapped_vec.into_inner().into_iter().next() {
                Some(template) => Ok(JsonAdvanced(template)),
                None => Err(UpdateTemplateError {
                    message: "No se encontró ningún Template".to_string(),
                    status_code: StatusCode::NOT_FOUND,
                }),
            }
        })
}

#[web::delete("/{id}")]
async fn delete_template(
    path: Path<String>,
) -> Result<JsonAdvanced<Template>, DeleteTemplateError> {
    let template_type = path.into_inner();
    DeleteTemplateUseCase::execute(&template_type).await
}

#[web::get("/{id}")]
async fn get_one_template(
    path: Path<String>,
    query: QueryAdvanced<QueryFront<Template>>,
) -> Result<JsonAdvanced<Option<Template>>, GetTemplatesError> {
    let id = path.into_inner();
    GetOneTemplateUseCase::execute(query.0, id.as_str()).await
}
