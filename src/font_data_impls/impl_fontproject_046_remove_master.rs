use super::*;

impl FontProject {
    pub fn remove_master(&mut self, master_id: &str) -> bool {
        if self.masters.len() <= 1 || !self.masters.iter().any(|master| master.id == master_id) {
            return false;
        }
        let fallback = self
            .masters
            .iter()
            .find(|master| master.id != master_id)
            .map(|master| master.id.clone())
            .expect("at least one master remains");
        self.masters.retain(|master| master.id != master_id);
        let remaining_axis_tags: std::collections::HashSet<String> = self
            .masters
            .iter()
            .flat_map(|master| master.axes.keys().cloned())
            .collect();
        self.axis_names
            .retain(|tag, _| remaining_axis_tags.contains(tag));
        for glyph in self.glyphs.values_mut() {
            glyph.layers.remove(master_id);
            glyph.master_guidelines.remove(master_id);
        }
        self.vertical_metrics_by_master.remove(master_id);
        self.metrics_by_master.remove(master_id);
        self.kerning_by_master.remove(master_id);
        self.guidelines_by_master.remove(master_id);
        for masters in self.background_images.values_mut() {
            masters.remove(master_id);
        }
        self.background_images
            .retain(|_, masters| !masters.is_empty());
        for masters in self.background_opacities.values_mut() {
            masters.remove(master_id);
        }
        self.background_opacities
            .retain(|_, masters| !masters.is_empty());
        for masters in self.background_transforms.values_mut() {
            masters.remove(master_id);
        }
        self.background_transforms
            .retain(|_, masters| !masters.is_empty());
        if self.default_master_id == master_id {
            self.default_master_id = fallback;
        }
        true
    }
}
