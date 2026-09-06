use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn align_left_side_bearing(&mut self) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let Some(min_x) = min_projected_outline_x(
            &self.project,
            &name,
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut std::collections::HashSet::new(),
        ) else {
            return;
        };
        if min_x.abs() <= f64::EPSILON {
            return;
        }
        let shift = -min_x;
        if let Some(glyph) = self.project.glyphs.get_mut(&name) {
            glyph.translate_geometry(shift, 0.0);
            glyph.width += shift;
            self.save_state();
            self.status_message = "左余白を0に揃えました".to_string();
        }
    }
}
