use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn apply_selected_node_action(&mut self, action: NodeAction) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let nodes = if !self.canvas.selected_nodes.is_empty() {
            self.canvas.selected_nodes.clone()
        } else if let Some(contour_index) = self.canvas.selected_contour {
            self.canvas
                .selected_points
                .iter()
                .map(|&point_index| (contour_index, point_index))
                .collect()
        } else {
            return;
        };
        if nodes.is_empty() {
            return;
        }
        let result = self
            .project
            .glyphs
            .get_mut(&name)
            .map(|glyph| match action {
                NodeAction::Smooth => glyph.set_smooth_nodes_all_layers(&nodes, true),
                NodeAction::Corner => glyph.set_smooth_nodes_all_layers(&nodes, false),
                NodeAction::ToggleCurve => glyph.toggle_curve_nodes_all_layers(&nodes),
            });
        match result {
            Some(Ok(())) => {
                self.save_state();
                self.status_message = match action {
                    NodeAction::Smooth => "スムーズノードにしました".to_string(),
                    NodeAction::Corner => "コーナーノードにしました".to_string(),
                    NodeAction::ToggleCurve => "オン/オフ曲線を切り替えました".to_string(),
                };
            }
            Some(Err(error)) => self.status_message = error,
            None => {}
        }
    }
}
