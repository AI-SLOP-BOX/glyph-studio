use super::*;

impl FontProject {
    pub fn switch_master(&mut self, from_id: &str, to_id: &str) {
        if from_id == to_id {
            return;
        }
        let from_exists = self.masters.iter().any(|master| master.id == from_id);
        if from_exists {
            self.kerning_by_master
                .insert(from_id.to_string(), self.kerning.clone());
            self.guidelines_by_master
                .insert(from_id.to_string(), self.guidelines.clone());
        }
        self.kerning = self
            .kerning_by_master
            .get(to_id)
            .cloned()
            .unwrap_or_else(|| self.kerning.clone());
        self.kerning_by_master
            .entry(to_id.to_string())
            .or_insert_with(|| self.kerning.clone());
        self.guidelines = self
            .guidelines_by_master
            .get(to_id)
            .cloned()
            .unwrap_or_else(|| self.guidelines.clone());
        self.guidelines_by_master
            .entry(to_id.to_string())
            .or_insert_with(|| self.guidelines.clone());
        for glyph in self.glyphs.values_mut() {
            if from_exists {
                glyph.switch_layer(from_id, to_id);
            } else {
                glyph.switch_layer(to_id, to_id);
            }
        }
    }
}
