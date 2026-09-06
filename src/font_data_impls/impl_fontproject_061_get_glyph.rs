use super::*;

impl FontProject {
    pub fn get_glyph(&self, name: &str) -> Option<&GlyphData> {
        self.glyphs.get(name)
    }
}
