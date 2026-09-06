use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn flip_selection(&mut self, horizontal: bool) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let component_indices = self.selected_component_indices();
        let Some(glyph) = self.project.glyphs.get_mut(&name) else {
            return;
        };
        if !component_indices.is_empty() {
            if self.edit_all_masters {
                for index in component_indices {
                    if let Err(error) = glyph.reflect_component_all_layers(index, horizontal) {
                        self.status_message = error;
                        return;
                    }
                }
            } else {
                for index in component_indices {
                    if let Some(component) = glyph.components.get_mut(index) {
                        if horizontal {
                            component.x_scale = -component.x_scale;
                            component.xy_scale = -component.xy_scale;
                        } else {
                            component.yx_scale = -component.yx_scale;
                            component.y_scale = -component.y_scale;
                        }
                    }
                }
            }
            self.save_state();
            return;
        }
        let Some(ci) = self.canvas.selected_contour else {
            return;
        };
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
            match glyph.reflect_nodes_all_layers(&nodes, horizontal) {
                Ok(()) => self.save_state(),
                Err(error) => self.status_message = error,
            }
            return;
        }
        let points: Vec<(f64, f64)> = nodes
            .iter()
            .filter_map(|&(node_ci, pi)| {
                glyph
                    .contours
                    .get(node_ci)
                    .and_then(|contour| contour.points.get(pi))
                    .map(|point| (point.x, point.y))
            })
            .collect();
        if points.is_empty() {
            return;
        }
        // Match the usual font-editor transform behavior: reflect around the
        // selection bounding box, not around the arithmetic mean of nodes.
        let min_x = points.iter().map(|(x, _)| *x).fold(f64::INFINITY, f64::min);
        let max_x = points
            .iter()
            .map(|(x, _)| *x)
            .fold(f64::NEG_INFINITY, f64::max);
        let min_y = points.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
        let max_y = points
            .iter()
            .map(|(_, y)| *y)
            .fold(f64::NEG_INFINITY, f64::max);
        let center = ((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
        for (node_ci, pi) in nodes {
            if let Some(point) = glyph
                .contours
                .get_mut(node_ci)
                .and_then(|contour| contour.points.get_mut(pi))
            {
                if horizontal {
                    point.x = center.0 - (point.x - center.0);
                } else {
                    point.y = center.1 - (point.y - center.1);
                }
            }
        }
        for contour in &mut glyph.contours {
            contour.repair_smooth_handles();
        }
        self.save_state();
    }
}
