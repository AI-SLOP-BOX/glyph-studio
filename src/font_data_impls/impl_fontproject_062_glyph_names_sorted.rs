use super::*;

impl FontProject {
    pub fn glyph_names_sorted(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .glyph_order
            .iter()
            .filter_map(|name| self.glyphs.contains_key(name).then_some(name.as_str()))
            .collect();
        let mut remaining: Vec<&str> = self
            .glyphs
            .keys()
            .filter(|name| !self.glyph_order.iter().any(|ordered| ordered == *name))
            .map(String::as_str)
            .collect();
        remaining.sort();
        names.extend(remaining);
        names
    }
}
