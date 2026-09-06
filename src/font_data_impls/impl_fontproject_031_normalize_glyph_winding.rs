use super::*;

impl FontProject {
    /// Normalizes outer contours and nested counters in every layer.
    pub fn normalize_glyph_winding(&mut self, names: &[String]) -> usize {
        let mut changed = 0;
        for name in names {
            let Some(glyph) = self.glyphs.get_mut(name) else {
                continue;
            };
            let mut glyph_changed = Self::normalize_contour_list(&mut glyph.contours);
            for layer in glyph.layers.values_mut() {
                glyph_changed |= Self::normalize_contour_list(&mut layer.contours);
            }
            if glyph_changed {
                changed += 1;
            }
        }
        changed
    }
}
