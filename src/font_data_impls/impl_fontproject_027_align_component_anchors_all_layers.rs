use super::*;

impl FontProject {
    /// Aligns a component against matching anchors in the active geometry and
    /// in every saved master layer.
    pub fn align_component_anchors_all_layers(
        &mut self,
        glyph_name: &str,
        component_index: usize,
    ) -> bool {
        let mut changed = self.align_component_anchors(glyph_name, component_index);
        let master_ids: Vec<String> = self
            .glyphs
            .get(glyph_name)
            .into_iter()
            .flat_map(|glyph| glyph.layers.keys().cloned())
            .collect();
        for master_id in master_ids {
            let mut snapshot = self.clone();
            for glyph in snapshot.glyphs.values_mut() {
                if let Some(layer) = glyph.layers.get(&master_id).cloned() {
                    glyph.width = layer.width;
                    glyph.contours = layer.contours;
                    glyph.components = layer.components;
                    glyph.anchors = layer.anchors;
                }
            }
            if snapshot.align_component_anchors(glyph_name, component_index) {
                if let Some(component) = snapshot
                    .glyphs
                    .get(glyph_name)
                    .and_then(|glyph| glyph.components.get(component_index))
                    .cloned()
                {
                    if let Some(layer) = self
                        .glyphs
                        .get_mut(glyph_name)
                        .and_then(|glyph| glyph.layers.get_mut(&master_id))
                    {
                        if let Some(target) = layer.components.get_mut(component_index) {
                            target.x_offset = component.x_offset;
                            target.y_offset = component.y_offset;
                            changed = true;
                        }
                    }
                }
            }
        }
        changed
    }
}
