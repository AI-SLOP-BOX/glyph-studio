use super::*;

impl FontProject {
    /// Duplicates one contour in the authored geometry and every saved master
    /// layer. The copy uses the authored contour, matching the active editor.
    pub fn duplicate_contour_all_layers(
        &mut self,
        glyph_name: &str,
        contour_index: usize,
    ) -> Option<usize> {
        let contour = self
            .glyphs
            .get(glyph_name)
            .and_then(|glyph| glyph.contours.get(contour_index))
            .cloned()?;
        self.add_contour_all_layers(glyph_name, contour)
    }
}
