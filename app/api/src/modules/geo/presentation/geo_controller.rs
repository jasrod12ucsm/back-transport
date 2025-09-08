use std::sync::Arc;

use ac_struct_back::schemas::auth::user::user::UserConfigError;
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::web::{self, types::State};

use crate::{
    modules::geo::domain::{
        cases::get_ids_polygon_use_case::{GetIdsPolygonUseCase, GetIdsPolygonUseCaseTrait},
        data::{
            get_ids_polygon_response::GetIdsPolygonResponse,
            point_collection_dto::PointCollectionDto,
        },
    },
    utils::domain::models::grcp_message::GrpcMessage,
};

#[web::post("get_ids_in_polygon")]
async fn get_ids_in_polygon(
    dto: common::utils::ntex_private::extractors::json::JsonAdvanced<PointCollectionDto>,
    grcp: State<Arc<GrpcMessage>>,
) -> Result<JsonAdvanced<GetIdsPolygonResponse>, UserConfigError> {
    GetIdsPolygonUseCase::new()
        .get_ids(dto.into_inner(), grcp.get_ref())
        .await
}
