use super::*;

impl FontProject {
    pub fn normalize_glyph_order(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.glyph_order
            .retain(|name| self.glyphs.contains_key(name) && seen.insert(name.clone()));
        let mut missing: Vec<String> = self
            .glyphs
            .keys()
            .filter(|name| !seen.contains(*name))
            .cloned()
            .collect();
        missing.sort();
        self.glyph_order.extend(missing);
    }
}
