use super::*;

impl FontProject {
    /// Removes one component from the authored geometry and every saved master
    /// layer. Refuses partial removal so master component indices stay aligned.
    pub fn remove_component_all_layers(
        &mut self,
        glyph_name: &str,
        component_index: usize,
    ) -> Result<(), String> {
        let Some(glyph) = self.glyphs.get_mut(glyph_name) else {
            return Err("対象グリフがありません".to_string());
        };
        if component_index >= glyph.components.len()
            || glyph
                .layers
                .values()
                .any(|layer| component_index >= layer.components.len())
        {
            return Err("マスター間で部品構造が一致しません".to_string());
        }
        glyph.components.remove(component_index);
        for layer in glyph.layers.values_mut() {
            layer.components.remove(component_index);
        }
        Ok(())
    }
}
