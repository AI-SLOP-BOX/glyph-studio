use super::*;

impl FontProject {
    /// Duplicates one component in the glyph and every saved master layer.
    pub fn duplicate_component_all_layers(
        &mut self,
        glyph_name: &str,
        component_index: usize,
    ) -> bool {
        let Some(component) = self
            .glyphs
            .get(glyph_name)
            .and_then(|glyph| glyph.components.get(component_index))
            .cloned()
        else {
            return false;
        };
        let Some(glyph) = self.glyphs.get_mut(glyph_name) else {
            return false;
        };
        glyph.components.push(component.clone());
        for layer in glyph.layers.values_mut() {
            layer.components.push(component.clone());
        }
        true
    }
}
