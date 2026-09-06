use super::*;

impl FontProject {
    /// Copies one master layer to every other master, preserving glyph metadata.
    /// This is useful when adding a new master that should initially match a
    /// finished master before making weight/width-specific edits.
    pub fn copy_master_to_all(&mut self, source_master_id: &str) -> usize {
        let target_ids: Vec<String> = self
            .masters
            .iter()
            .filter(|master| master.id != source_master_id)
            .map(|master| master.id.clone())
            .collect();
        let source_is_default = source_master_id == self.default_master_id;
        let source_kerning = self
            .kerning_by_master
            .get(source_master_id)
            .cloned()
            .or_else(|| source_is_default.then(|| self.kerning.clone()))
            .unwrap_or_default();
        let source_guidelines = self
            .guidelines_by_master
            .get(source_master_id)
            .cloned()
            .or_else(|| source_is_default.then(|| self.guidelines.clone()))
            .unwrap_or_default();
        for target_id in target_ids {
            self.kerning_by_master
                .insert(target_id.clone(), source_kerning.clone());
            self.guidelines_by_master
                .insert(target_id, source_guidelines.clone());
        }
        let names: Vec<String> = self.glyphs.keys().cloned().collect();
        self.copy_master_to_all_for_glyphs(source_master_id, names.iter().map(String::as_str))
    }
}
