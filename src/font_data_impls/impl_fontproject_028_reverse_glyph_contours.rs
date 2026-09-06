use super::*;

impl FontProject {
    pub fn reverse_glyph_contours(&mut self, names: &[String]) -> usize {
        let mut changed = 0;
        for name in names {
            if let Some(glyph) = self.glyphs.get_mut(name) {
                for contour in &mut glyph.contours {
                    contour.reverse_direction();
                }
                for layer in glyph.layers.values_mut() {
                    for contour in &mut layer.contours {
                        contour.reverse_direction();
                    }
                }
                changed += 1;
            }
        }
        changed
    }
}
