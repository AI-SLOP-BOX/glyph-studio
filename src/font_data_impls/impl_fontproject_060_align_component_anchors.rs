use super::*;

impl FontProject {
    /// Aligns a component's underscored anchors to matching parent anchors.
    pub fn align_component_anchors(&mut self, glyph_name: &str, component_index: usize) -> bool {
        let (base_name, component) = match self.glyphs.get(glyph_name).and_then(|glyph| {
            glyph
                .components
                .get(component_index)
                .map(|component| (component.base.clone(), component.clone()))
        }) {
            Some(value) => value,
            None => return false,
        };
        let parent_anchors = self
            .glyphs
            .get(glyph_name)
            .map(|glyph| glyph.anchors.clone())
            .unwrap_or_default();
        let child_anchors = self.anchors_for_glyph(&base_name);
        let mut delta = (0.0, 0.0);
        let mut matches = 0.0;
        for child in &child_anchors {
            let Some(name) = child.name.strip_prefix('_') else {
                continue;
            };
            let Some(parent) = parent_anchors.iter().find(|anchor| anchor.name == name) else {
                continue;
            };
            let x = component.x_scale * child.x + component.xy_scale * child.y + component.x_offset;
            let y = component.yx_scale * child.x + component.y_scale * child.y + component.y_offset;
            delta.0 += parent.x - x;
            delta.1 += parent.y - y;
            matches += 1.0;
        }
        if matches == 0.0 {
            return false;
        }
        let (dx, dy) = (delta.0 / matches, delta.1 / matches);
        if let Some(glyph) = self.glyphs.get_mut(glyph_name) {
            let component = &mut glyph.components[component_index];
            component.x_offset += dx;
            component.y_offset += dy;
            return true;
        }
        false
    }
}
