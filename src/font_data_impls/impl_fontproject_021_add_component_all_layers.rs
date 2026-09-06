use super::*;

impl FontProject {
    /// Adds one component to the authored geometry and every saved master
    /// layer, keeping composite component indices aligned for interpolation.
    pub fn add_component_all_layers(
        &mut self,
        glyph_name: &str,
        component: GlyphComponent,
    ) -> Option<usize> {
        let glyph = self.glyphs.get_mut(glyph_name)?;
        let index = glyph.components.len();
        glyph.components.push(component.clone());
        for layer in glyph.layers.values_mut() {
            layer.components.push(component.clone());
        }
        Some(index)
    }
}
