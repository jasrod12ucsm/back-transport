use serde::{Deserialize, Serialize};

use crate::utils::infrastructure::grcp_client::geocache::IdAndLocation;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IdLocationModel {
    pub id: String,
    pub latitude: f64,
    pub longitude: f64,
}

impl From<IdAndLocation> for IdLocationModel {
    fn from(id_and_location: IdAndLocation) -> Self {
        Self {
            id: id_and_location.id,
            latitude: id_and_location.latitude,
            longitude: id_and_location.longitude,
        }
    }
}
