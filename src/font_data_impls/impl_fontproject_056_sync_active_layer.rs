use super::*;

impl FontProject {
    /// Persists the currently active geometry into its selected master layer.
    /// This keeps edits made before a master switch or export from becoming stale.
    pub fn sync_active_layer(&mut self, master_id: &str) {
        for glyph in self.glyphs.values_mut() {
            glyph
                .layers
                .insert(master_id.to_string(), glyph.layer_snapshot());
            let guides = glyph
                .master_guidelines
                .get(master_id)
                .cloned()
                .unwrap_or_else(|| glyph.guidelines.clone());
            glyph.guidelines = guides.clone();
            glyph
                .master_guidelines
                .insert(master_id.to_string(), guides);
        }
        self.kerning_by_master
            .insert(master_id.to_string(), self.kerning.clone());
        let guides = self
            .guidelines_by_master
            .get(master_id)
            .cloned()
            .unwrap_or_else(|| self.guidelines.clone());
        self.guidelines = guides.clone();
        self.guidelines_by_master
            .insert(master_id.to_string(), guides);
    }
}
