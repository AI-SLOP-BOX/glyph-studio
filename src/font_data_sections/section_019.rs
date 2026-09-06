
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ColorLayerTransform {
    #[serde(default = "default_color_transform_scale")]
    pub xx: f64,
    #[serde(default)]
    pub yx: f64,
    #[serde(default)]
    pub xy: f64,
    #[serde(default = "default_color_transform_scale")]
    pub yy: f64,
    #[serde(default)]
    pub dx: f64,
    #[serde(default)]
    pub dy: f64,
}
