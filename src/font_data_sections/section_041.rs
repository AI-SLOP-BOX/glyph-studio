
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlyphLayer {
    pub width: f64,
    pub contours: Vec<Contour>,
    pub components: Vec<GlyphComponent>,
    #[serde(default)]
    pub anchors: Vec<GlyphAnchor>,
}
