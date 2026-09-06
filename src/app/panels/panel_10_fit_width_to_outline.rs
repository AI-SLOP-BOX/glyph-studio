use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn fit_width_to_outline(&mut self) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let Some(max_x) = max_projected_outline_x(
            &self.project,
            &name,
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut std::collections::HashSet::new(),
        ) else {
            return;
        };
        let Some(glyph) = self.project.glyphs.get_mut(&name) else {
            return;
        };
        if max_x >= 0.0 && (glyph.width - max_x).abs() > f64::EPSILON {
            glyph.width = max_x;
            self.save_state();
            self.status_message = "字幅をアウトラインの右端に合わせました".to_string();
        }
    }
}
