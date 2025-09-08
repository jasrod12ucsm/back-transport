use serde::{Deserialize, Serialize};

use crate::{
    modules::geo::domain::models::id_location_model::IdLocationModel,
    utils::infrastructure::grcp_client::geocache::IdAndLocation,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetIdsPolygonResponse {
    pub ids: Vec<IdLocationModel>,
}
