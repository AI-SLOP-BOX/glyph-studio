use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn align_selection(&mut self, horizontal: bool) {
        if !self.selected_component_indices().is_empty() {
            self.align_selected_components(horizontal);
            return;
        }
        let (Some(name), Some(ci)) = (self.current_glyph.clone(), self.canvas.selected_contour)
        else {
            return;
        };
        if let Some(glyph) = self.project.glyphs.get_mut(&name) {
            let nodes: Vec<(usize, usize)> = if self.canvas.selected_nodes.is_empty() {
                self.canvas
                    .selected_points
                    .iter()
                    .map(|&pi| (ci, pi))
                    .collect()
            } else {
                self.canvas.selected_nodes.clone()
            };
            if self.edit_all_masters {
                match glyph.align_nodes_all_layers(&nodes, horizontal) {
                    Ok(()) => self.save_state(),
                    Err(error) => self.status_message = error,
                }
                return;
            }
            let values: Vec<f64> = nodes
                .iter()
                .filter_map(|&(node_ci, pi)| {
                    glyph
                        .contours
                        .get(node_ci)
                        .and_then(|c| c.points.get(pi))
                        .map(|p| if horizontal { p.y } else { p.x })
                })
                .collect();
            if values.is_empty() {
                return;
            }
            let target = values.iter().copied().sum::<f64>() / values.len() as f64;
            for (node_ci, pi) in nodes {
                if let Some(point) = glyph
                    .contours
                    .get_mut(node_ci)
                    .and_then(|c| c.points.get_mut(pi))
                {
                    if horizontal {
                        point.y = target;
                    } else {
                        point.x = target;
                    }
                }
            }
            for contour in &mut glyph.contours {
                contour.repair_smooth_handles();
            }
            self.save_state();
        }
    }
}
