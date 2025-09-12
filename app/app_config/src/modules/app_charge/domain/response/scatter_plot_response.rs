use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

#[derive(Serialize, Deserialize, Debug)]
pub struct ScatterPlotResponse {
    pub features: Vec<ScatterPlotFeature>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScatterPlotFeature {
    pub name: String,
    pub id: RecordId,
    pub scatter: Vec<ContentScatter>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContentScatter {
    pub content_scatter: Vec<ScatterPlot>,
    pub out: RecordId,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScatterPlot {
    pub x: f64,
    pub y: f64,
}
