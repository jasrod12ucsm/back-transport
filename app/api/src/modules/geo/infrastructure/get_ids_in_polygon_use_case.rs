use ac_struct_back::schemas::auth::user::user::UserConfigError;
use common::utils::ntex_private::extractors::json::JsonAdvanced;
use ntex::http::StatusCode;

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

#[async_trait::async_trait]
impl GetIdsPolygonUseCaseTrait for GetIdsPolygonUseCase {
    async fn get_ids<'a>(
        &self,
        dto: PointCollectionDto,
        grcp: &GrpcMessage,
    ) -> Result<JsonAdvanced<GetIdsPolygonResponse>, UserConfigError> {
        //first get the points same that message in grpc

        //message GetDevicesPolygonRequest {
        //  double latitude_1 = 1;
        //  double longitude_1 = 2;
        //  double latitude_2 = 3;
        //  double longitude_2 = 4;
        //  double latitude_3 = 5;
        //  double longitude_3 = 6;
        //  double latitude_4 = 7;
        //  double longitude_4 = 8;
        //}
        if dto.points.len() != 4 {
            return Err(UserConfigError {
                message: "Invalid points".to_string(),
                status_code: StatusCode::BAD_REQUEST,
            });
        }
        let latitude1 = dto.points[0].y;
        let longitude1 = dto.points[0].x;
        let latitude2 = dto.points[1].y;
        let longitude2 = dto.points[1].x;
        let latitude3 = dto.points[2].y;
        let longitude3 = dto.points[2].x;
        let latitude4 = dto.points[3].y;
        let longitude4 = dto.points[3].x;
        println!("latitude1: {:?}", latitude1);
        let ids = grcp
            .get_grcp_message(
                latitude1, latitude2, latitude3, latitude4, longitude1, longitude2, longitude3,
                longitude4,
            )
            .await
            .map_err(|_| UserConfigError {
                message: "Error getting ids".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;

        Ok(JsonAdvanced(GetIdsPolygonResponse {
            ids: ids.into_iter().map(Into::into).collect(),
        }))
    }
}
