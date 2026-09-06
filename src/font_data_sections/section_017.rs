
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorLayer {
    pub glyph: String,
    pub palette_index: u16,
    #[serde(default)]
    pub gradient: Option<ColorGradient>,
    #[serde(default = "default_color_layer_alpha")]
    pub alpha: f64,
}
