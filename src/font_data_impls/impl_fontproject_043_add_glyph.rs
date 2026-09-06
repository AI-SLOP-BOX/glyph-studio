use super::*;

impl FontProject {
    pub fn add_glyph(&mut self, name: String, unicode: Option<u32>) {
        if !self.glyphs.contains_key(&name) {
            self.glyphs
                .insert(name.clone(), GlyphData::new(name.clone(), unicode));
            self.glyph_order.push(name);
        }
    }
}
