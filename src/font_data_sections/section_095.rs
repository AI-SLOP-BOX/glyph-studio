
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PointType {
    OnCurve,
    OffCurve,
}
