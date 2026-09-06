
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AxisMappingPoint {
    /// Input and output are normalized coordinates in the -1.0..=1.0 range.
    pub input: f64,
    pub output: f64,
}
