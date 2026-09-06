
/// A named location in the variable-font design space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FontInstance {
    pub name: String,
    #[serde(default)]
    pub axes: HashMap<String, f64>,
    #[serde(default = "default_instance_weight")]
    pub weight: f64,
    #[serde(default = "default_instance_width")]
    pub width: f64,
}
