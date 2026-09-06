
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ContourPoint {
    pub x: f64,
    pub y: f64,
    pub point_type: PointType,
    pub smooth: bool,
}
