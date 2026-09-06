use super::*;

impl CanvasState {
    pub fn draw_layer(
        &self,
        painter: &egui::Painter,
        layer: &crate::font_data::GlyphLayer,
        origin: Pos2,
        color: Color32,
    ) {
        for (index, contour) in layer.contours.iter().enumerate() {
            self.draw_contour(painter, contour, origin, color, index);
        }
    }

    pub fn draw_master_overlay(
        &self,
        painter: &egui::Painter,
        project: &FontProject,
        glyph_name: &str,
        master_id: &str,
        origin: Pos2,
        color: Color32,
    ) {
        self.draw_colored_glyph_recursive_inner(
            painter,
            project,
            glyph_name,
            master_id,
            origin,
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            color,
            &mut Vec::new(),
        );
    }

    pub fn draw_conditional_layer(
        &self,
        painter: &egui::Painter,
        project: &FontProject,
        layer: &crate::font_data::GlyphLayer,
        master_id: &str,
        origin: Pos2,
        color: Color32,
    ) {
        self.draw_layer(painter, layer, origin, color);
        for component in &layer.components {
            self.draw_colored_glyph_recursive_inner(
                painter,
                project,
                &component.base,
                master_id,
                origin,
                compose_transform((1.0, 0.0, 0.0, 1.0, 0.0, 0.0), component),
                color,
                &mut Vec::new(),
            );
        }
    }

    pub(super) fn draw_glyph_recursive_inner(
        &self,
        painter: &egui::Painter,
        project: &FontProject,
        glyph_name: &str,
        origin: Pos2,
        parent: (f64, f64, f64, f64, f64, f64),
        stack: &mut Vec<String>,
    ) {
        if stack.iter().any(|name| name == glyph_name) {
            return;
        }
        let Some(glyph) = project.glyphs.get(glyph_name) else {
            return;
        };
        stack.push(glyph_name.to_string());
        for (index, contour) in glyph.contours.iter().enumerate() {
            let transformed = Contour {
                points: contour
                    .points
                    .iter()
                    .map(|point| ContourPoint {
                        x: parent.0 * point.x + parent.2 * point.y + parent.4,
                        y: parent.1 * point.x + parent.3 * point.y + parent.5,
                        ..*point
                    })
                    .collect(),
            };
            self.draw_contour(painter, &transformed, origin, Color32::WHITE, index);
        }
        for component in &glyph.components {
            let transform = compose_transform(parent, component);
            self.draw_glyph_recursive_inner(
                painter,
                project,
                &component.base,
                origin,
                transform,
                stack,
            );
        }
        stack.pop();
    }

    pub fn draw_selection(&self, painter: &egui::Painter) {
        if let Some(rect) = self.selection_rect {
            painter.rect_filled(
                rect,
                0.0,
                Color32::from_rgba_premultiplied(80, 140, 255, 35),
            );
            painter.rect_stroke(
                rect,
                0.0,
                Stroke::new(1.0_f32, Color32::from_rgb(100, 170, 255)),
                egui::StrokeKind::Inside,
            );
        }
    }

    pub fn draw_ruler(&self, painter: &egui::Painter, origin: Pos2) {
        let (Some(start), Some(end)) = (self.ruler_start, self.ruler_end) else {
            return;
        };
        painter.line_segment(
            [start, end],
            Stroke::new(1.5_f32, Color32::from_rgb(255, 190, 40)),
        );
        let (x1, y1) = self.screen_to_glyph(start, origin);
        let (x2, y2) = self.screen_to_glyph(end, origin);
        let label = format!(
            "Δx {:.1}  Δy {:.1}  {:.1}",
            x2 - x1,
            y2 - y1,
            ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt()
        );
        painter.text(
            end + Vec2::new(8.0, -8.0),
            egui::Align2::LEFT_BOTTOM,
            label,
            egui::FontId::monospace(11.0),
            Color32::from_rgb(255, 220, 100),
        );
    }
}
