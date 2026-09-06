
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlyphData {
    pub name: String,
    pub unicode: Option<u32>,
    #[serde(default)]
    pub unicodes: Vec<u32>,
    pub width: f64,
    #[serde(default)]
    pub left_kerning_group: String,
    #[serde(default)]
    pub right_kerning_group: String,
    /// Optional metric-key references such as `=H`. The left/right key uses
    /// the corresponding side bearing of the referenced glyph when applied.
    #[serde(default)]
    pub left_metrics_key: String,
    #[serde(default)]
    pub right_metrics_key: String,
    #[serde(default)]
    pub anchors: Vec<GlyphAnchor>,
    pub contours: Vec<Contour>,
    pub components: Vec<GlyphComponent>,
    #[serde(default)]
    pub layers: HashMap<String, GlyphLayer>,
    #[serde(default)]
    pub guidelines: Vec<Guideline>,
    /// Per-master glyph guides. `guidelines` remains the active-layer view
    /// for backwards-compatible callers and file formats.
    #[serde(default)]
    pub master_guidelines: HashMap<String, Vec<Guideline>>,
}
