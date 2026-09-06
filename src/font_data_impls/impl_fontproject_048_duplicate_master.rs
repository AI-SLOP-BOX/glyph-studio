use super::*;

impl FontProject {
    /// Duplicates a master, including its design-space metadata and every
    /// glyph layer. The new master is inserted immediately after its source.
    pub fn duplicate_master(&mut self, source_master_id: &str) -> Option<String> {
        let source_index = self
            .masters
            .iter()
            .position(|master| master.id == source_master_id)?;
        let source = self.masters[source_index].clone();
        let mut suffix = 1;
        let new_id = loop {
            let candidate = format!("{}.copy{}", source.id, suffix);
            if !self.masters.iter().any(|master| master.id == candidate) {
                break candidate;
            }
            suffix += 1;
        };
        let mut duplicate = source.clone();
        duplicate.id = new_id.clone();
        duplicate.name = format!("{} Copy", source.name);
        self.masters.insert(source_index + 1, duplicate);
        let source_is_default = source_master_id == self.default_master_id;

        if let Some(pairs) = self.kerning_by_master.get(source_master_id).cloned() {
            self.kerning_by_master.insert(new_id.clone(), pairs);
        } else if source_is_default {
            self.kerning_by_master
                .insert(new_id.clone(), self.kerning.clone());
        }
        if let Some(guides) = self.guidelines_by_master.get(source_master_id).cloned() {
            self.guidelines_by_master.insert(new_id.clone(), guides);
        } else if source_is_default {
            self.guidelines_by_master
                .insert(new_id.clone(), self.guidelines.clone());
        }

        for glyph in self.glyphs.values_mut() {
            if let Some(layer) = glyph
                .layers
                .get(source_master_id)
                .cloned()
                .or_else(|| source_is_default.then(|| glyph.layer_snapshot()))
            {
                glyph.layers.insert(new_id.clone(), layer);
            }
            let source_guidelines = glyph
                .master_guidelines
                .get(source_master_id)
                .cloned()
                .or_else(|| source_is_default.then(|| glyph.guidelines.clone()))
                .unwrap_or_default();
            glyph
                .master_guidelines
                .insert(new_id.clone(), source_guidelines);
        }
        Some(new_id)
    }
}
