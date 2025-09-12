use ac_struct_back::{
    schemas::config::proyect::proyect::{GetOneProjectError, Proyect},
    utils::domain::front_query::QueryFront,
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

pub struct GetOneProyectUseCase;
#[async_trait::async_trait]
pub trait GetOneProyectUseCasePublic {
    async fn execute(
        query: QueryFront<Proyect>,
        id: &str,
    ) -> Result<JsonAdvanced<Option<Proyect>>, GetOneProjectError>;
}
