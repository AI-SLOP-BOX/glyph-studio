use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn translate_selected_components_by(&mut self, deltas: &[(usize, f64, f64)]) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let Some(glyph) = self.project.glyphs.get_mut(&name) else {
            return;
        };
        for &(index, dx, dy) in deltas {
            if self.edit_all_masters {
                if let Err(error) = glyph.translate_component_all_layers(index, dx, dy) {
                    self.status_message = error;
                    return;
                }
            } else if let Some(component) = glyph.components.get_mut(index) {
                component.x_offset += dx;
                component.y_offset += dy;
            }
        }
        self.save_state();
    }
}
