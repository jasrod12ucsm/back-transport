use ac_struct_back::{
    schemas::config::template_type::template_type::{
        DeleteTemplateTypeError, TemplateType, TemplateTypeError, TemplateTypesNotFoundError,
        UpdateTemplateTypeError, updatetemplatetypedtotemplatetype::UpdateTemplateTypeDto,
    },
    utils::domain::{
        front_query::{QueryFront, UpdateRequestBuilderFront},
        query::{Query, UpdateRequest, UpdateRequestBuilder},
    },
};
use common::utils::ntex_private::extractors::{json::JsonAdvanced, query_advanced::QueryAdvanced};
use ntex::web::{self, types::Path};

use crate::modules::template_type::domain::use_cases::{
    create_template_type_use_case::{CreateTemplateTypeUseCase, CreateTemplateTypeUseCasePublic},
    delete_template_type_use_case::{DeleteTemplateTypeUseCase, DeleteTemplateTypeUseCasePublic},
    get_one_template_type_use_case::{GetOneTemplateTypeUseCase, GetOneTemplateTypeUseCasePublic},
    get_templates_type_use_case::{GetTemplatesTypeUseCase, GetTemplatesTypeUseCasePublic},
    update_request_use_case::{UpdateTemplateTypeUseCase, UpdateTemplateTypeUseCasePublic},
};

#[web::get("/")]
async fn get_template_types(
    query: QueryAdvanced<QueryFront<TemplateType>>,
) -> Result<JsonAdvanced<Vec<TemplateType>>, TemplateTypesNotFoundError> {
    GetTemplatesTypeUseCase::execute(query.0).await
}

#[web::post("/")]
async fn create_template_type(
    dto:JsonAdvanced<ac_struct_back::schemas::config::template_type::template_type::createtemplatetypedtotemplatetype::CreateTemplateTypeDto>,
) -> Result<JsonAdvanced<TemplateType>, TemplateTypeError> {
    let case = CreateTemplateTypeUseCase {};
    case.execute(dto.0).await
}

#[web::patch("/")]
async fn update_template_type(
    dto: JsonAdvanced<UpdateTemplateTypeDto>,
    query: QueryAdvanced<UpdateRequestBuilderFront<TemplateType>>,
) -> Result<JsonAdvanced<Vec<TemplateType>>, UpdateTemplateTypeError> {
    UpdateTemplateTypeUseCase::execute(None, query.0, dto.0).await
}

#[web::delete("/{id}")]
async fn delete_template_type(
    path: Path<String>,
) -> Result<JsonAdvanced<TemplateType>, DeleteTemplateTypeError> {
    let template_type = path.into_inner();
    DeleteTemplateTypeUseCase::execute(&template_type).await
}

#[web::get("/{id}")]
async fn get_one_template_type(
    path: Path<String>,
    query: QueryAdvanced<QueryFront<TemplateType>>,
) -> Result<JsonAdvanced<Option<TemplateType>>, TemplateTypesNotFoundError> {
    let id = path.into_inner();
    GetOneTemplateTypeUseCase::execute(query.0, id.as_str()).await
}

#[web::put("/{id}")]
async fn update_template_type_by_id(
    path: Path<String>,
    dto: JsonAdvanced<UpdateTemplateTypeDto>,
    query: QueryAdvanced<UpdateRequestBuilderFront<TemplateType>>,
) -> Result<JsonAdvanced<Vec<TemplateType>>, UpdateTemplateTypeError> {
    let id = path.into_inner();
    UpdateTemplateTypeUseCase::execute(Some(id), query.0, dto.0).await
}
