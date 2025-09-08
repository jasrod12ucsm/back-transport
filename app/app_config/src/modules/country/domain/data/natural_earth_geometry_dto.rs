use ac_struct_back::utils::domain::surreal::{Geometry, GeometryMultiPolygon, GeometryPolygon};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaturalEarthGeometryDto {
    #[serde(rename = "type")]
    pub type_: String,
    pub features: Vec<NaturalEarthFeatureDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaturalEarthFeatureDto {
    pub properties: NaturalEarthPropertiesDto,
    pub geometry: Geometry,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NaturalEarthPropertiesDto {
    #[serde(rename = "ISO_A2")]
    pub cca2: String,
}
