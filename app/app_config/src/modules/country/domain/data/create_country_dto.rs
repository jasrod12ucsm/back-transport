use std::collections::HashMap;

use serde_json::Value;
use validator::Validate;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateCountryDto {
    pub name: NameCountry,
    pub cca2: String,
    pub cca3: String,
    pub ccn3: String,
    //vector pero de longitud 2
    pub latlng: Vec<f64>,
    pub flag: String,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NameCountry {
    pub common: String,
    #[serde(flatten)]
    pub value: HashMap<String, Value>,
}
