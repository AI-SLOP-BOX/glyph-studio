use super::*;

impl FontProject {
    pub fn translate_glyphs(&mut self, names: &[String], dx: f64, dy: f64) -> usize {
        if !dx.is_finite() || !dy.is_finite() {
            return 0;
        }
        let mut changed = 0;
        for name in names {
            if let Some(glyph) = self.glyphs.get_mut(name) {
                glyph.translate_geometry(dx, dy);
                changed += 1;
            }
        }
        changed
    }
}
