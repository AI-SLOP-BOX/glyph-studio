use super::*;

impl GlyphData {
    /// Returns the guides belonging to a master, falling back to the legacy
    /// active-layer field for projects created before per-master guides.
    pub fn guidelines_for_master(&self, master_id: &str) -> &[Guideline] {
        self.master_guidelines
            .get(master_id)
            .map(Vec::as_slice)
            .unwrap_or(&self.guidelines)
    }
}
