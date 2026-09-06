
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorGradientStop {
    pub offset: f64,
    pub palette_index: u16,
    #[serde(default = "default_gradient_alpha")]
    pub alpha: f64,
}
