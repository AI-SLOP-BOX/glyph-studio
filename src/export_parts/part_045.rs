
/// Collect issues that would make an exported font ambiguous or unusable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub message: String,
    pub glyph_name: Option<String>,
}
