use ac_struct_back::schemas::config::proyect::proyect::{DeleteProjectError, Proyect};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

pub struct DeleteProyectUseCase;
#[async_trait::async_trait]
pub trait DeleteProyectUseCasePublic {
    async fn execute(id: &str) -> Result<JsonAdvanced<Proyect>, DeleteProjectError>;
}
