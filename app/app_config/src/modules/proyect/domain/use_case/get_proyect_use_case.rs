use ac_struct_back::{
    schemas::config::proyect::proyect::{GetProjectError, Proyect},
    utils::domain::front_query::QueryFront,
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;

pub struct GetProyectsUseCase;
#[async_trait::async_trait]
pub trait GetProyectsUseCasePublic {
    async fn execute(
        query: QueryFront<Proyect>,
    ) -> Result<JsonAdvanced<Vec<Proyect>>, GetProjectError>;
}
