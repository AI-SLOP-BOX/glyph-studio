use super::*;

impl FontProject {
    /// Appends one contour to the authored geometry and every saved master
    /// layer, preserving contour indices for interpolation.
    pub fn add_contour_all_layers(&mut self, glyph_name: &str, contour: Contour) -> Option<usize> {
        let glyph = self.glyphs.get_mut(glyph_name)?;
        let index = glyph.contours.len();
        glyph.contours.push(contour.clone());
        for layer in glyph.layers.values_mut() {
            layer.contours.push(contour.clone());
        }
        Some(index)
    }
}
