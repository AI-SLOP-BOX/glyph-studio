use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn show_canvas_context_menu(&mut self, response: &egui::Response, rect: egui::Rect) {
        response.context_menu(|ui| {
            if ui.button("選択を解除").clicked() {
                self.clear_canvas_selection();
                ui.close_menu();
            }
            if ui.button("グリフの全ノードを選択").clicked() {
                if let Some(name) = &self.current_glyph {
                    if let Some(glyph) = self.project.glyphs.get(name) {
                        self.canvas.selected_nodes = glyph
                            .contours
                            .iter()
                            .enumerate()
                            .flat_map(|(ci, contour)| {
                                (0..contour.points.len()).map(move |pi| (ci, pi))
                            })
                            .collect();
                        self.canvas.selected_contour = glyph.contours.first().map(|_| 0);
                        self.canvas.selected_points = glyph
                            .contours
                            .first()
                            .map(|contour| (0..contour.points.len()).collect())
                            .unwrap_or_default();
                    }
                }
                ui.close_menu();
            }
            ui.separator();
            if ui
                .add_enabled(
                    !self.canvas.selected_nodes.is_empty(),
                    egui::Button::new("スムーズノードにする"),
                )
                .clicked()
            {
                self.apply_selected_node_action(NodeAction::Smooth);
                ui.close_menu();
            }
            if ui
                .add_enabled(
                    !self.canvas.selected_nodes.is_empty(),
                    egui::Button::new("コーナーノードにする"),
                )
                .clicked()
            {
                self.apply_selected_node_action(NodeAction::Corner);
                ui.close_menu();
            }
            ui.separator();
            if ui.button("全体表示").clicked() {
                self.fit_current_glyph_to_canvas(rect);
                ui.close_menu();
            }
        });
    }
}
