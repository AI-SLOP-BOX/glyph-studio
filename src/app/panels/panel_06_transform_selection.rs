use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn transform_selection(&mut self, scale: f64, angle: f64) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let component_indices = self.selected_component_indices();
        if component_indices.len() > 1 {
            let (sin, cos) = angle.sin_cos();
            let mut changed = false;
            if let Some(glyph) = self.project.glyphs.get_mut(&name) {
                if self.edit_all_masters {
                    for component_index in component_indices {
                        match glyph.transform_component_all_layers(component_index, scale, angle) {
                            Ok(()) => changed = true,
                            Err(error) => self.status_message = error,
                        }
                    }
                } else {
                    for component_index in component_indices {
                        if let Some(component) = glyph.components.get_mut(component_index) {
                            let a = component.x_scale;
                            let b = component.xy_scale;
                            let c = component.yx_scale;
                            let d = component.y_scale;
                            component.x_scale = scale * (cos * a - sin * b);
                            component.xy_scale = scale * (sin * a + cos * b);
                            component.yx_scale = scale * (cos * c - sin * d);
                            component.y_scale = scale * (sin * c + cos * d);
                            changed = true;
                        }
                    }
                }
            }
            if changed {
                self.save_state();
            }
            return;
        }
        if let Some(glyph) = self.project.glyphs.get_mut(&name) {
            if let Some(index) = self.canvas.selected_component {
                if self.edit_all_masters {
                    match glyph.transform_component_all_layers(index, scale, angle) {
                        Ok(()) => self.save_state(),
                        Err(error) => self.status_message = error,
                    }
                    return;
                }
                if let Some(component) = glyph.components.get_mut(index) {
                    let (sin, cos) = angle.sin_cos();
                    let a = component.x_scale;
                    let b = component.xy_scale;
                    let c = component.yx_scale;
                    let d = component.y_scale;
                    component.x_scale = scale * (cos * a - sin * b);
                    component.xy_scale = scale * (sin * a + cos * b);
                    component.yx_scale = scale * (cos * c - sin * d);
                    component.y_scale = scale * (sin * c + cos * d);
                    self.save_state();
                }
                return;
            }
            let Some(contour_index) = self.canvas.selected_contour else {
                return;
            };
            if self.edit_all_masters {
                let nodes: Vec<(usize, usize)> = if self.canvas.selected_nodes.is_empty() {
                    self.canvas
                        .selected_points
                        .iter()
                        .map(|&point_index| (contour_index, point_index))
                        .collect()
                } else {
                    self.canvas.selected_nodes.clone()
                };
                if !nodes.is_empty() {
                    match glyph.transform_nodes_all_layers(&nodes, scale, angle) {
                        Ok(()) => self.save_state(),
                        Err(error) => self.status_message = error,
                    }
                }
                return;
            }
            let changed = if !self.canvas.selected_nodes.is_empty() {
                self.canvas.transform_selected_nodes(glyph, scale, angle)
            } else {
                self.canvas
                    .transform_selected(glyph, contour_index, scale, angle)
            };
            if changed {
                self.save_state();
            }
        }
    }
}
