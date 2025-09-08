use std::sync::Arc;

use ac_struct_back::{
    import::macro_import::async_trait,
    schemas::auth::user::user::{
        UserConfigError, registeruserdtouserconfig::RegisterUserDto,
        userconfigiduserconfig::UserConfigId,
    },
};
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::web::types::State;
use surrealdb::{RecordId, Surreal, engine::any::Any};

use crate::{
    modules::geo::domain::data::{
        get_ids_polygon_response::GetIdsPolygonResponse, point_collection_dto::PointCollectionDto,
    },
    utils::domain::models::grcp_message::GrpcMessage,
};

pub struct GetIdsPolygonUseCase;

impl GetIdsPolygonUseCase {
    pub fn new() -> Self {
        GetIdsPolygonUseCase {}
    }
}
#[async_trait::async_trait]
pub trait GetIdsPolygonUseCaseTrait {
    async fn get_ids<'a>(
        &self,
        user_dto: PointCollectionDto,
        grcp: &GrpcMessage,
    ) -> Result<JsonAdvanced<GetIdsPolygonResponse>, UserConfigError>;
}
