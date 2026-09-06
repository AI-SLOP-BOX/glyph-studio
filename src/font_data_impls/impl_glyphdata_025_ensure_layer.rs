use super::*;

impl GlyphData {
    pub fn ensure_layer(&mut self, master_id: &str) {
        if self.layers.is_empty() {
            self.layers
                .insert(master_id.to_string(), self.layer_snapshot());
        }
        self.master_guidelines
            .entry(master_id.to_string())
            .or_insert_with(|| self.guidelines.clone());
    }
}
