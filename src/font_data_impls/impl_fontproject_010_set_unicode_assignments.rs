use super::*;

impl FontProject {
    /// Assigns primary Unicode values, removing conflicts from other glyphs.
    /// Returns the number of glyph records changed.
    pub fn set_unicode_assignments(&mut self, assignments: &[(String, u32)]) -> usize {
        let mut changed = 0;
        for (name, codepoint) in assignments {
            if char::from_u32(*codepoint).is_none() || !self.glyphs.contains_key(name) {
                continue;
            }
            for (other_name, glyph) in &mut self.glyphs {
                if other_name == name {
                    continue;
                }
                let mut glyph_changed = false;
                if glyph.unicode == Some(*codepoint) {
                    glyph.unicode = None;
                    glyph_changed = true;
                }
                let aliases_before = glyph.unicodes.len();
                glyph.unicodes.retain(|alias| *alias != *codepoint);
                glyph_changed |= aliases_before != glyph.unicodes.len();
                changed += usize::from(glyph_changed);
            }
            if let Some(glyph) = self.glyphs.get_mut(name) {
                if glyph.unicode != Some(*codepoint) {
                    glyph.unicode = Some(*codepoint);
                    changed += 1;
                }
                glyph.unicodes.retain(|alias| *alias != *codepoint);
            }
        }
        changed
    }
}
