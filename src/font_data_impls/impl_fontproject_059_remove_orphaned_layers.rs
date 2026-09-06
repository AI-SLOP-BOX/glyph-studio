use super::*;

impl FontProject {
    /// Removes layers whose master no longer exists and returns the number removed.
    pub fn remove_orphaned_layers(&mut self) -> usize {
        let valid: std::collections::HashSet<String> = self
            .masters
            .iter()
            .map(|master| master.id.clone())
            .collect();
        let mut removed = 0;
        for glyph in self.glyphs.values_mut() {
            let before = glyph.layers.len();
            glyph
                .layers
                .retain(|master_id, _| valid.contains(master_id));
            glyph
                .master_guidelines
                .retain(|master_id, _| valid.contains(master_id));
            removed += before - glyph.layers.len();
        }
        let before = self.kerning_by_master.len();
        self.kerning_by_master
            .retain(|master_id, _| valid.contains(master_id));
        removed += before - self.kerning_by_master.len();
        let before = self.guidelines_by_master.len();
        self.guidelines_by_master
            .retain(|master_id, _| valid.contains(master_id));
        removed += before - self.guidelines_by_master.len();
        removed
    }
}
