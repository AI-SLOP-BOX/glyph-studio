use super::*;

impl GlyphData {
    /// Returns the editable guide list for a master, upgrading legacy data
    /// lazily when that master has no explicit guide list yet.
    pub fn guidelines_for_master_mut(&mut self, master_id: &str) -> &mut Vec<Guideline> {
        if !self.master_guidelines.contains_key(master_id) {
            self.master_guidelines
                .insert(master_id.to_string(), self.guidelines.clone());
        }
        self.master_guidelines
            .get_mut(master_id)
            .expect("guide entry inserted")
    }
}
