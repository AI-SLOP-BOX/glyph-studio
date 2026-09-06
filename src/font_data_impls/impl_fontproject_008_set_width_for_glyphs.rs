use super::*;

impl FontProject {
    /// Sets the advance width for each existing glyph in `names`.
    /// Returns the number of glyphs whose value changed.
    pub fn set_width_for_glyphs(&mut self, names: &[String], width: f64) -> usize {
        if !width.is_finite() || width < 0.0 {
            return 0;
        }
        let mut changed = 0;
        for name in names {
            if let Some(glyph) = self.glyphs.get_mut(name) {
                if (glyph.width - width).abs() > f64::EPSILON {
                    glyph.width = width;
                    for layer in glyph.layers.values_mut() {
                        layer.width = width;
                    }
                    changed += 1;
                }
            }
        }
        changed
    }
}
