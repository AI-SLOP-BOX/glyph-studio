use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn distribute_selection(&mut self, horizontal: bool) {
        if !self.selected_component_indices().is_empty() {
            self.distribute_selected_components(horizontal);
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
                match glyph.distribute_nodes_all_layers(&nodes, horizontal) {
                    Ok(()) => self.save_state(),
                    Err(error) => self.status_message = error,
                }
                return;
            }
            let mut values: Vec<(f64, usize, usize)> = nodes
                .iter()
                .filter_map(|&(node_ci, pi)| {
                    glyph
                        .contours
                        .get(node_ci)
                        .and_then(|c| c.points.get(pi))
                        .map(|p| (if horizontal { p.x } else { p.y }, node_ci, pi))
                })
                .collect();
            if values.len() < 3 {
                return;
            }
            values.sort_by(|a, b| a.0.total_cmp(&b.0));
            let first = values.first().unwrap().0;
            let last = values.last().unwrap().0;
            let step = (last - first) / (values.len() - 1) as f64;
            for (index, (_, node_ci, pi)) in values.into_iter().enumerate() {
                if let Some(point) = glyph
                    .contours
                    .get_mut(node_ci)
                    .and_then(|c| c.points.get_mut(pi))
                {
                    if horizontal {
                        point.x = first + step * index as f64;
                    } else {
                        point.y = first + step * index as f64;
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
