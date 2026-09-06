
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Guideline {
    pub x: f64,
    pub y: f64,
    #[serde(default)]
    pub angle: f64,
    #[serde(default)]
    pub name: String,
}
