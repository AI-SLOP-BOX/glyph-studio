use super::*;

impl FontProject {
    pub fn normalize_masters(&mut self) {
        if self.masters.is_empty() {
            self.masters = default_masters();
        }
        let mut seen = std::collections::HashSet::new();
        self.masters
            .retain(|master| !master.id.trim().is_empty() && seen.insert(master.id.clone()));
        if self.masters.is_empty() {
            self.masters = default_masters();
        }
        let default_id = self.masters[0].id.clone();
        if !self
            .masters
            .iter()
            .any(|master| master.id == self.default_master_id)
        {
            self.default_master_id = default_id.clone();
        }
        let axis_tags: std::collections::HashSet<String> = self
            .masters
            .iter()
            .flat_map(|master| master.axes.keys().cloned())
            .collect();
        let axis_defaults = self
            .masters
            .iter()
            .find(|master| master.id == self.default_master_id)
            .or_else(|| self.masters.first())
            .map(|master| master.axes.clone())
            .unwrap_or_default();
        for master in &mut self.masters {
            for tag in &axis_tags {
                let default = axis_defaults.get(tag).copied().unwrap_or(0.0);
                master.axes.entry(tag.clone()).or_insert(default);
            }
        }
        self.axis_names.retain(|tag, _| axis_tags.contains(tag));
        self.guidelines_by_master
            .entry(default_id.clone())
            .or_insert_with(|| self.guidelines.clone());
        for glyph in self.glyphs.values_mut() {
            glyph.ensure_layer(&default_id);
        }
    }
}
