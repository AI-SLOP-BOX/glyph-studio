use super::*;

impl FontProject {
    pub fn kerning_for_glyphs(&self, left: &str, right: &str) -> Option<f64> {
        self.kerning_source_for_glyphs(left, right)
            .map(|(_, value)| value)
    }
}
