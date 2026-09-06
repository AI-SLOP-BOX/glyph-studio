use super::*;

impl GlyphData {
    pub fn switch_layer(&mut self, from_id: &str, to_id: &str) {
        self.ensure_layer(from_id);
        self.layers
            .insert(from_id.to_string(), self.layer_snapshot());
        self.master_guidelines
            .insert(from_id.to_string(), self.guidelines.clone());
        if let Some(layer) = self.layers.get(to_id).cloned() {
            self.width = layer.width;
            self.contours = layer.contours;
            self.components = layer.components;
            self.anchors = layer.anchors;
            self.guidelines = self
                .master_guidelines
                .get(to_id)
                .cloned()
                .unwrap_or_default();
            self.master_guidelines
                .entry(to_id.to_string())
                .or_insert_with(|| self.guidelines.clone());
        } else {
            let layer = self.layer_snapshot();
            self.layers.insert(to_id.to_string(), layer);
            self.master_guidelines
                .insert(to_id.to_string(), self.guidelines.clone());
        }
    }
}
