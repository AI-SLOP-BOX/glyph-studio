use super::*;

impl FontProject {
    pub fn duplicate_glyph(&mut self, source_name: &str) -> Option<String> {
        let mut duplicate = self.glyphs.get(source_name)?.clone();
        let mut suffix = 1;
        let new_name = loop {
            let candidate = format!("{source_name}.copy{suffix}");
            if !self.glyphs.contains_key(&candidate) {
                break candidate;
            }
            suffix += 1;
        };
        duplicate.name = new_name.clone();
        duplicate.unicode = None;
        duplicate.unicodes.clear();
        self.glyph_order.push(new_name.clone());
        self.glyphs.insert(new_name.clone(), duplicate);
        if let Some(layers) = self.color_layers.get(source_name).cloned() {
            self.color_layers.insert(new_name.clone(), layers);
        }
        if let Some(transforms) = self.color_layer_transforms.get(source_name).cloned() {
            self.color_layer_transforms
                .insert(new_name.clone(), transforms);
        }
        if let Some(layers) = self.conditional_layers.get(source_name).cloned() {
            self.conditional_layers.insert(new_name.clone(), layers);
        }
        if let Some(metrics) = self.vertical_metrics.get(source_name).copied() {
            self.vertical_metrics.insert(new_name.clone(), metrics);
        }
        if let Some(images) = self.background_images.get(source_name).cloned() {
            self.background_images.insert(new_name.clone(), images);
        }
        if let Some(opacities) = self.background_opacities.get(source_name).cloned() {
            self.background_opacities
                .insert(new_name.clone(), opacities);
        }
        if let Some(transforms) = self.background_transforms.get(source_name).cloned() {
            self.background_transforms
                .insert(new_name.clone(), transforms);
        }
        for metrics in self.vertical_metrics_by_master.values_mut() {
            if let Some(value) = metrics.get(source_name).copied() {
                metrics.insert(new_name.clone(), value);
            }
        }
        Some(new_name)
    }
}
