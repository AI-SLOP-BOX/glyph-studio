
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlyphComponent {
    pub base: String,
    pub x_scale: f64,
    pub xy_scale: f64,
    pub yx_scale: f64,
    pub y_scale: f64,
    pub x_offset: f64,
    pub y_offset: f64,
}
