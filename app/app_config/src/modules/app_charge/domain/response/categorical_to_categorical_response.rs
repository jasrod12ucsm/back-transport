use ac_struct_back::schemas::config::categorical_to_categorical::CategoricalContent;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

#[derive(Serialize, Deserialize, Debug)]
pub struct CategoricalPlotResponse {
    pub features: Vec<CategoricalPlotFeature>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CategoricalPlotFeature {
    pub name: String,
    pub id: RecordId,
    pub scatter: Vec<ContentScatter>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContentScatter {
    pub content: CategoricalContent,
    pub out: RecordId,
    pub name: String,
}
