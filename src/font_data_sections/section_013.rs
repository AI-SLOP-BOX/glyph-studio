
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColorGradient {
    pub start_palette_index: u16,
    pub end_palette_index: u16,
    #[serde(default)]
    pub kind: ColorGradientKind,
    #[serde(default)]
    pub extend: ColorGradientExtend,
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    #[serde(default)]
    pub x2: f64,
    #[serde(default)]
    pub y2: f64,
    #[serde(default)]
    pub stops: Vec<ColorGradientStop>,
    #[serde(default = "default_gradient_radius")]
    pub radius0: f64,
    #[serde(default = "default_gradient_radius")]
    pub radius1: f64,
    #[serde(default)]
    pub start_angle: f64,
    #[serde(default = "default_gradient_end_angle")]
    pub end_angle: f64,
}
