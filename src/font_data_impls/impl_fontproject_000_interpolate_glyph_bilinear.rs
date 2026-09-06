use super::*;

impl FontProject {
    /// Interpolates a glyph in a complete rectangular two-axis master set.
    pub fn interpolate_glyph_bilinear(
        &self,
        glyph_name: &str,
        axis_x: &str,
        axis_y: &str,
        target_x: f64,
        target_y: f64,
    ) -> Option<GlyphLayer> {
        let (indices, (x_factor, y_factor)) =
            find_bilinear_masters(&self.masters, axis_x, axis_y, target_x, target_y)?;
        let glyph = self.glyphs.get(glyph_name)?;
        let layers = indices.map(|index| glyph.layers.get(&self.masters[index].id));
        layers[0]?.interpolate_bilinear(layers[1]?, layers[2]?, layers[3]?, x_factor, y_factor)
    }
}
