use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn prepare_canvas_state(&mut self) {
        if self.canvas.selected_component.is_none() {
            self.canvas.selected_components.clear();
        }
        let referenced_backgrounds: std::collections::HashSet<String> = self
            .project
            .background_images
            .values()
            .flat_map(|masters| masters.values())
            .filter(|path| !path.trim().is_empty())
            .cloned()
            .collect();
        self.background_cache
            .retain(|path, _| referenced_backgrounds.contains(path));
    }

    pub(crate) fn show_canvas_cursor(
        &self,
        painter: &egui::Painter,
        response: &egui::Response,
        origin: egui::Pos2,
    ) {
        let Some(mouse_pos) = response.hover_pos() else {
            return;
        };
        painter.circle_filled(
            mouse_pos,
            3.0,
            Color32::from_rgba_premultiplied(255, 255, 255, 100),
        );
        if self.current_tool != Tool::Hand {
            let (cursor_x, cursor_y) = self.canvas.screen_to_glyph(mouse_pos, origin);
            painter.text(
                mouse_pos + Vec2::new(10.0, 12.0),
                egui::Align2::LEFT_TOP,
                format!("{cursor_x:.0}, {cursor_y:.0}"),
                egui::FontId::monospace(10.0),
                Color32::from_rgba_premultiplied(220, 225, 235, 190),
            );
        }
    }

    pub(crate) fn update_space_tool(&mut self, ctx: &egui::Context) {
        let space_down = ctx.input(|input| input.key_down(Key::Space));
        if space_down {
            if self.space_previous_tool.is_none() && self.current_tool != Tool::Hand {
                self.space_previous_tool = Some(self.current_tool);
                self.current_tool = Tool::Hand;
            }
        } else if let Some(previous_tool) = self.space_previous_tool.take() {
            self.current_tool = previous_tool;
        }
    }

    pub(crate) fn apply_canvas_view_requests(
        &mut self,
        rect: egui::Rect,
        zoom_delta: Option<(f32, egui::Pos2)>,
        reset_view_requested: bool,
        fit_view_requested: bool,
    ) {
        if let Some((delta, mouse_pos)) = zoom_delta {
            self.canvas.zoom_at(delta, mouse_pos, rect.center());
        }
        if reset_view_requested {
            self.canvas.zoom = 1.0;
            self.canvas.pan = Vec2::ZERO;
        }
        if fit_view_requested {
            self.fit_current_glyph_to_canvas(rect);
        }
    }
}
