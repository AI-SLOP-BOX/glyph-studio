use super::*;

impl FontProject {
    /// Sets left and right side bearings by translating authored geometry and
    /// recalculating advance width for every available master layer.
    /// Sets left and right kerning groups for existing glyphs.
    pub fn set_kerning_groups(
        &mut self,
        names: &[String],
        left_group: &str,
        right_group: &str,
    ) -> Result<usize, String> {
        let left_group = left_group.trim();
        let right_group = right_group.trim();
        if left_group.chars().any(char::is_whitespace)
            || right_group.chars().any(char::is_whitespace)
        {
            return Err("カーニンググループ名に空白は使えません".into());
        }
        for name in names {
            if !self.glyphs.contains_key(name) {
                return Err(format!("グリフ '{}' がありません", name));
            }
        }
        let mut changed = 0;
        for name in names {
            let Some(glyph) = self.glyphs.get_mut(name) else {
                continue;
            };
            if glyph.left_kerning_group != left_group || glyph.right_kerning_group != right_group {
                glyph.left_kerning_group = left_group.to_string();
                glyph.right_kerning_group = right_group.to_string();
                changed += 1;
            }
        }
        Ok(changed)
    }
}
