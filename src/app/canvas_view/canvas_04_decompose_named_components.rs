use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn decompose_named_components(&mut self, names: &[String]) -> usize {
        let mut changed = 0;
        for name in names {
            let Some(components) = self.project.glyphs.get(name).map(|g| g.components.clone())
            else {
                continue;
            };
            if components.is_empty() {
                continue;
            }
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
                .get(name)
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
            if let Some(glyph) = self.project.glyphs.get_mut(name) {
                if !contours.is_empty() {
                    glyph.contours.extend(contours);
                    glyph.components.clear();
                    for (master_id, decomposed) in layer_contours {
                        if let Some(layer) = glyph.layers.get_mut(&master_id) {
                            layer.contours.extend(decomposed);
                            layer.components.clear();
                        }
                    }
                    changed += 1;
                }
            }
        }
        changed
    }
}
