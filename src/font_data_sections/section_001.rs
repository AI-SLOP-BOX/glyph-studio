
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnicodeVariationSequence {
    pub base: u32,
    pub selector: u32,
    pub glyph: String,
}
