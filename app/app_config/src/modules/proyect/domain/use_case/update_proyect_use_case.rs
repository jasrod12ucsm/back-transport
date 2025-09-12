use ac_struct_back::{
    schemas::config::proyect::proyect::{Proyect, UpdateProjectError},
    utils::domain::front_query::UpdateRequestBuilderFront,
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

use crate::modules::proyect::domain::data::update_proyect_dto::UpdateProyectDto;

pub struct UpdateProyectUseCase;
#[async_trait::async_trait]
pub trait UpdateProyectUseCasePublic {
    async fn execute(
        id: Option<String>,
        request: UpdateRequestBuilderFront<Proyect>,
        dto: UpdateProyectDto,
    ) -> Result<JsonAdvanced<Vec<Proyect>>, UpdateProjectError>;
}
