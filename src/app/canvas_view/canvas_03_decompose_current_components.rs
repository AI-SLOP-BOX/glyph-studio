use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn decompose_current_components(&mut self) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let Some(components) = self
            .project
            .glyphs
            .get(&name)
            .map(|glyph| glyph.components.clone())
        else {
            return;
        };
        let mut contours = Vec::new();
        let mut visiting = std::collections::HashSet::new();
        for component in &components {
            collect_decomposed_contours(
                &self.project,
                &component.base,
                component_transform(component),
                &mut visiting,
                &mut contours,
            );
        }
        let master_ids: Vec<String> = self
            .project
            .glyphs
            .get(&name)
            .into_iter()
            .flat_map(|glyph| glyph.layers.keys().cloned())
            .collect();
        let mut layer_contours = Vec::new();
        for master_id in master_ids {
            let mut decomposed = Vec::new();
            let mut visiting = std::collections::HashSet::new();
            for component in &components {
                collect_decomposed_contours_for_master(
                    &self.project,
                    &component.base,
                    &master_id,
                    component_transform(component),
                    &mut visiting,
                    &mut decomposed,
                );
            }
            layer_contours.push((master_id, decomposed));
        }
        if let Some(glyph) = self.project.glyphs.get_mut(&name) {
            if contours.is_empty() {
                return;
            }
            glyph.contours.extend(contours);
            glyph.components.clear();
            for (master_id, decomposed) in layer_contours {
                if let Some(layer) = glyph.layers.get_mut(&master_id) {
                    layer.contours.extend(decomposed);
                    layer.components.clear();
                }
            }
            self.canvas.selected_contour = None;
            self.canvas.selected_points.clear();
            self.canvas.selected_nodes.clear();
            self.save_state();
            self.status_message = "コンポーネントを輪郭化しました".to_string();
        }
    }
}
