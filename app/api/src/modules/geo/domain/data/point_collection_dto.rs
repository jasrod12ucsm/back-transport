use serde::{Deserialize, Serialize};

use crate::utils::domain::point::Point;

#[derive(Debug, Serialize, Deserialize)]
pub struct PointCollectionDto<T = f64> {
    pub points: Vec<Point<T>>,
}
