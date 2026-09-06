
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConditionalLayer {
    pub id: String,
    pub conditions: HashMap<String, AxisRange>,
    pub layer: GlyphLayer,
}
