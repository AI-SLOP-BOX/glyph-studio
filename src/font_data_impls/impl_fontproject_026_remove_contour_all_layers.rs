use super::*;

impl FontProject {
    /// Removes one contour from the authored geometry and every saved master
    /// layer through the project-level API used by clipboard actions.
    pub fn remove_contour_all_layers(
        &mut self,
        glyph_name: &str,
        contour_index: usize,
    ) -> Result<(), String> {
        self.glyphs
            .get_mut(glyph_name)
            .ok_or_else(|| "対象グリフがありません".to_string())?
            .remove_contour_all_layers(contour_index)
    }
}
