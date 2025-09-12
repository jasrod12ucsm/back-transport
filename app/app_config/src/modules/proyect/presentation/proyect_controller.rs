use ac_struct_back::{
    schemas::config::proyect::proyect::{
        CreateProjectError, DeleteProjectError, GetOneProjectError, GetProjectError, Proyect,
        UpdateProjectError,
    },
    utils::domain::front_query::{QueryFront, UpdateRequestBuilderFront},
};
use common::utils::ntex_private::extractors::{json::JsonAdvanced, query_advanced::QueryAdvanced};
use ntex::web::{self, types::Path};

use crate::modules::proyect::domain::{
    data::{create_proyect_dto::CreateProyectDto, update_proyect_dto::UpdateProyectDto},
    use_case::{
        create_proyect_use_case::{CreateProyectUseCase, CreateProyectUseCasePublic},
        delete_proyect_use_case::{DeleteProyectUseCase, DeleteProyectUseCasePublic},
        get_one_proyect_use_case::{GetOneProyectUseCase, GetOneProyectUseCasePublic},
        get_proyect_use_case::{GetProyectsUseCase, GetProyectsUseCasePublic},
        update_proyect_use_case::{UpdateProyectUseCase, UpdateProyectUseCasePublic},
    },
};

#[web::get("/")]
async fn get_templates(
    query: QueryAdvanced<QueryFront<Proyect>>,
) -> Result<JsonAdvanced<Vec<Proyect>>, GetProjectError> {
    GetProyectsUseCase::execute(query.0).await
}

#[web::post("/")]
async fn create_template(
    dto: JsonAdvanced<CreateProyectDto>,
) -> Result<JsonAdvanced<Proyect>, CreateProjectError> {
    let case = CreateProyectUseCase {};
    case.execute(dto.0).await
}
#[web::patch("/")]
async fn update_template(
    dto: JsonAdvanced<UpdateProyectDto>,
    query: QueryAdvanced<UpdateRequestBuilderFront<Proyect>>,
) -> Result<JsonAdvanced<Vec<Proyect>>, UpdateProjectError> {
    UpdateProyectUseCase::execute(None, query.0, dto.0).await
}

#[web::patch("/{id}")]
async fn update_template_by_id(
    path: Path<String>,
    dto: JsonAdvanced<UpdateProyectDto>,
    query: QueryAdvanced<UpdateRequestBuilderFront<Proyect>>,
) -> Result<JsonAdvanced<Proyect>, UpdateProjectError> {
    let id = path.into_inner();
    UpdateProyectUseCase::execute(Some(id), query.0, dto.0)
        .await
        .and_then(|wrapped_vec| {
            // Suponiendo que wrapped_vec es JsonAdvanced<Vec<Template>>
            // y queremos obtener el primer Template, si existe
            match wrapped_vec.into_inner().into_iter().next() {
                Some(template) => Ok(JsonAdvanced(template)),
                None => Err(UpdateProjectError::NotFoundError),
            }
        })
}

#[web::delete("/{id}")]
async fn delete_template(path: Path<String>) -> Result<JsonAdvanced<Proyect>, DeleteProjectError> {
    let template_type = path.into_inner();
    DeleteProyectUseCase::execute(&template_type).await
}

#[web::get("/{id}")]
async fn get_one_template(
    path: Path<String>,
    query: QueryAdvanced<QueryFront<Proyect>>,
) -> Result<JsonAdvanced<Option<Proyect>>, GetOneProjectError> {
    let id = path.into_inner();
    GetOneProyectUseCase::execute(query.0, id.as_str()).await
}
