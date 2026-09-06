
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BackgroundImageTransform {
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    #[serde(default)]
    pub rotation: f32,
    #[serde(default)]
    pub flip_x: bool,
    #[serde(default)]
    pub flip_y: bool,
}
