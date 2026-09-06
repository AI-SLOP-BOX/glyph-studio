use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn handle_select_click(
        &mut self,
        ctx: &egui::Context,
        _response: &egui::Response,
        origin: egui::Pos2,
        mouse_pos: egui::Pos2,
    ) {
        if let Some(name) = &self.current_glyph {
            if let Some(glyph) = self.project.glyphs.get(name) {
                if let Some((ci, pi)) = self.canvas.hit_test(mouse_pos, glyph, origin) {
                    self.canvas.selected_component = None;
                    self.canvas.selected_components.clear();
                    let additive = ctx.input(|i| i.modifiers.shift);
                    if additive {
                        if let Some(pos) = self
                            .canvas
                            .selected_nodes
                            .iter()
                            .position(|&selected| selected == (ci, pi))
                        {
                            self.canvas.selected_nodes.remove(pos);
                        } else {
                            self.canvas.selected_nodes.push((ci, pi));
                        }
                    } else {
                        self.canvas.selected_nodes = vec![(ci, pi)];
                    }
                    self.canvas.selected_points = self
                        .canvas
                        .selected_nodes
                        .iter()
                        .filter_map(|&(selected_ci, selected_pi)| {
                            (selected_ci == ci).then_some(selected_pi)
                        })
                        .collect();
                    self.canvas.selected_contour = Some(ci);
                } else {
                    let component_hit = self
                        .current_glyph
                        .as_ref()
                        .and_then(|name| self.project.glyphs.get(name))
                        .and_then(|glyph| {
                            self.canvas
                                .hit_test_component(mouse_pos, &self.project, glyph, origin)
                        });
                    if let Some(component_index) = component_hit {
                        self.select_component(
                            component_index,
                            ctx.input(|input| input.modifiers.shift),
                        );
                    } else {
                        self.canvas.selected_component = None;
                        self.canvas.selected_components.clear();
                    }
                    self.canvas.selected_points.clear();
                    self.canvas.selected_nodes.clear();
                    self.canvas.selected_contour = None;
                }
            }
        }
    }
}
