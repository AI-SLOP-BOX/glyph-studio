use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn fit_current_glyph_to_canvas(&mut self, rect: egui::Rect) {
        let Some(name) = self.current_glyph.as_deref() else {
            return;
        };
        let bounds = self.project.outline_bounds_for_glyph(name);
        let Some((min_x, min_y, max_x, max_y)) = bounds else {
            self.canvas.zoom = 1.0;
            self.canvas.pan = Vec2::ZERO;
            return;
        };
        let width = (max_x - min_x).max(1.0) + 200.0;
        let height = (max_y - min_y).max(1.0) + 200.0;
        self.canvas.zoom = ((rect.width() as f64 / width).min(rect.height() as f64 / height))
            .clamp(0.05, 50.0) as f32;
        let center_x = (min_x + max_x) * 0.5;
        let center_y = (min_y + max_y) * 0.5;
        self.canvas.pan = Vec2::new(
            (-center_x * self.canvas.zoom as f64) as f32,
            (center_y * self.canvas.zoom as f64) as f32,
        );
    }
}
