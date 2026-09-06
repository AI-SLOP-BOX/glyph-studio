use super::*;

impl FontProject {
    /// Reorders one component in the authored geometry and every saved master
    /// layer by one position.
    pub fn move_component_all_layers(
        &mut self,
        glyph_name: &str,
        component_index: usize,
        delta: isize,
    ) -> Result<(), String> {
        let Some(glyph) = self.glyphs.get_mut(glyph_name) else {
            return Err("対象グリフがありません".to_string());
        };
        let target = component_index as isize + delta;
        if component_index >= glyph.components.len()
            || target < 0
            || target as usize >= glyph.components.len()
            || glyph.layers.values().any(|layer| {
                component_index >= layer.components.len()
                    || target as usize >= layer.components.len()
            })
        {
            return Err("マスター間で部品構造が一致しません".to_string());
        }
        let target = target as usize;
        glyph.components.swap(component_index, target);
        for layer in glyph.layers.values_mut() {
            layer.components.swap(component_index, target);
        }
        Ok(())
    }
}
