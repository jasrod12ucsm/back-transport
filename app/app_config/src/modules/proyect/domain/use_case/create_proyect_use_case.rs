use ac_struct_back::schemas::config::proyect::proyect::{CreateProjectError, Proyect};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

use crate::modules::proyect::domain::data::create_proyect_dto::CreateProyectDto;

pub struct CreateProyectUseCase;
#[async_trait::async_trait]
pub trait CreateProyectUseCasePublic {
    async fn execute(
        &self,
        dto: CreateProyectDto,
    ) -> Result<JsonAdvanced<Proyect>, CreateProjectError>;
}
