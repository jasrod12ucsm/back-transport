use ac_struct_back::utils::domain::surreal::{GeometryMultiPolygon, GeometryPolygon};

use crate::modules::country::domain::data::create_country_dto::NameCountry;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CountryToUpdate {
    pub name: NameCountry,
    pub cca2: String,
    pub cca3: String,
    pub ccn3: String,
    //vector pero de longitud 2
    pub geo_polygon: Option<GeometryPolygon>,
    pub geo_multi_polygon: Option<GeometryMultiPolygon>,
    pub latlng: Vec<f64>,
    pub flag: String,
}
