use super::*;

impl FontProject {
    pub fn remove_glyph(&mut self, name: &str) {
        self.glyphs.remove(name);
        self.glyph_order.retain(|item| item != name);
        self.vertical_metrics.remove(name);
        for metrics in self.vertical_metrics_by_master.values_mut() {
            metrics.remove(name);
        }
        self.vertical_metrics_by_master
            .retain(|_, metrics| !metrics.is_empty());
        self.color_layers.remove(name);
        self.color_layer_transforms.remove(name);
        self.conditional_layers.remove(name);
        self.background_images.remove(name);
        self.background_opacities.remove(name);
        self.background_transforms.remove(name);
        for layers in self.color_layers.values_mut() {
            layers.retain(|layer| layer.glyph != name);
        }
        self.color_layers.retain(|_, layers| !layers.is_empty());
        self.kerning
            .retain(|(left, right), _| left != name && right != name);
        for pairs in self.kerning_by_master.values_mut() {
            pairs.retain(|(left, right), _| left != name && right != name);
        }
        self.kerning_by_master.retain(|_, pairs| !pairs.is_empty());
        for glyph in self.glyphs.values_mut() {
            glyph.components.retain(|component| component.base != name);
            for layer in glyph.layers.values_mut() {
                layer.components.retain(|component| component.base != name);
            }
        }
        self.opentype_features =
            rewrite_feature_glyph_name(&self.opentype_features, name, ".notdef");
        self.opentype_classes = rewrite_feature_glyph_name(&self.opentype_classes, name, ".notdef");
        self.unicode_variation_sequences
            .retain(|sequence| sequence.glyph != name);
    }
}
