use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn translate_selected_nodes_by(&mut self, dx: f64, dy: f64) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let nodes = self.canvas.selected_nodes.clone();
        if nodes.is_empty() {
            return;
        }
        let Some(glyph) = self.project.glyphs.get_mut(&name) else {
            return;
        };
        if self.edit_all_masters {
            if let Err(error) = glyph.translate_nodes_all_layers(&nodes, dx, dy) {
                self.status_message = error;
                return;
            }
        } else {
            for (contour_index, contour) in glyph.contours.iter_mut().enumerate() {
                let points: Vec<usize> = nodes
                    .iter()
                    .filter_map(|&(selected_contour, point_index)| {
                        (selected_contour == contour_index).then_some(point_index)
                    })
                    .collect();
                if !points.is_empty() {
                    contour.translate_points(&points, dx, dy);
                }
            }
        }
        self.save_state();
        self.status_message = format!("{}個のノードを数値移動しました", nodes.len());
    }
}
