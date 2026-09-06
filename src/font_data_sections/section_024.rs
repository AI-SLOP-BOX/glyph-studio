
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AxisRange {
    pub min: Option<f64>,
    pub max: Option<f64>,
}
