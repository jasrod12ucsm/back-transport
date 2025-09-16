use ac_struct_back::schemas::config::{
    categorical_to_categorical::CategoricalContent, continous_to_categorical::ContinousContent,
};
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

#[derive(Serialize, Deserialize, Debug)]
pub struct ContinousCategoricalResponse {
    pub features: Vec<CategoricalPlotFeature>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CategoricalPlotFeature {
    pub name: String,
    pub id: RecordId,
    pub scatter: Vec<ContentCategorical>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContentCategorical {
    pub content: ContinousContent,
    pub out: RecordId,
    pub name: String,
}
