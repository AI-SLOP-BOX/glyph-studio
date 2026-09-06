use super::*;

impl FontProject {
    pub fn align_all_component_anchors(&mut self, names: &[String]) -> usize {
        let targets: Vec<(String, usize)> = names
            .iter()
            .flat_map(|name| {
                let count = self
                    .glyphs
                    .get(name)
                    .map(|glyph| glyph.components.len())
                    .unwrap_or(0);
                (0..count).map(|index| (name.clone(), index))
            })
            .collect();
        let mut changed = 0;
        for (name, index) in targets {
            if self.align_component_anchors_all_layers(&name, index) {
                changed += 1;
            }
        }
        changed
    }
}
