use super::*;

impl FontProject {
    pub fn rename_glyph(&mut self, old_name: &str, new_name: String) -> bool {
        if old_name == new_name || new_name.trim().is_empty() || self.glyphs.contains_key(&new_name)
        {
            return false;
        }
        let Some(mut glyph) = self.glyphs.remove(old_name) else {
            return false;
        };
        glyph.name = new_name.clone();
        self.glyphs.insert(new_name.clone(), glyph);
        for name in &mut self.glyph_order {
            if name == old_name {
                *name = new_name.clone();
            }
        }
        let kerning = std::mem::take(&mut self.kerning);
        self.kerning = kerning
            .into_iter()
            .map(|((left, right), value)| {
                (
                    (
                        if left == old_name {
                            new_name.clone()
                        } else {
                            left
                        },
                        if right == old_name {
                            new_name.clone()
                        } else {
                            right
                        },
                    ),
                    value,
                )
            })
            .collect();
        for pairs in self.kerning_by_master.values_mut() {
            let old_pairs = std::mem::take(pairs);
            *pairs = old_pairs
                .into_iter()
                .map(|((left, right), value)| {
                    (
                        (
                            if left == old_name {
                                new_name.clone()
                            } else {
                                left
                            },
                            if right == old_name {
                                new_name.clone()
                            } else {
                                right
                            },
                        ),
                        value,
                    )
                })
                .collect();
        }
        for other in self.glyphs.values_mut() {
            for component in &mut other.components {
                if component.base == old_name {
                    component.base = new_name.clone();
                }
            }
            for layer in other.layers.values_mut() {
                for component in &mut layer.components {
                    if component.base == old_name {
                        component.base = new_name.clone();
                    }
                }
            }
        }
        if let Some(metrics) = self.vertical_metrics.remove(old_name) {
            self.vertical_metrics.insert(new_name.clone(), metrics);
        }
        for metrics in self.vertical_metrics_by_master.values_mut() {
            if let Some(value) = metrics.remove(old_name) {
                metrics.insert(new_name.clone(), value);
            }
        }
        if let Some(layers) = self.color_layers.remove(old_name) {
            self.color_layers.insert(new_name.clone(), layers);
        }
        if let Some(transforms) = self.color_layer_transforms.remove(old_name) {
            self.color_layer_transforms
                .insert(new_name.clone(), transforms);
        }
        if let Some(layers) = self.conditional_layers.remove(old_name) {
            self.conditional_layers.insert(new_name.clone(), layers);
        }
        if let Some(images) = self.background_images.remove(old_name) {
            self.background_images.insert(new_name.clone(), images);
        }
        if let Some(opacities) = self.background_opacities.remove(old_name) {
            self.background_opacities
                .insert(new_name.clone(), opacities);
        }
        if let Some(transforms) = self.background_transforms.remove(old_name) {
            self.background_transforms
                .insert(new_name.clone(), transforms);
        }
        for layers in self.color_layers.values_mut() {
            for layer in layers {
                if layer.glyph == old_name {
                    layer.glyph = new_name.clone();
                }
            }
        }
        self.opentype_features =
            rewrite_feature_glyph_name(&self.opentype_features, old_name, &new_name);
        self.opentype_classes =
            rewrite_feature_glyph_name(&self.opentype_classes, old_name, &new_name);
        for sequence in &mut self.unicode_variation_sequences {
            if sequence.glyph == old_name {
                sequence.glyph = new_name.clone();
            }
        }
        true
    }
}
