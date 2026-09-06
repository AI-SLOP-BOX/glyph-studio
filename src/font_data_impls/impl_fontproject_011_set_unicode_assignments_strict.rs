use super::*;

impl FontProject {
    /// Unicode一括設定の厳格版。全グリフとコードポイントを検証してから適用する。
    pub fn set_unicode_assignments_strict(
        &mut self,
        assignments: &[(String, u32)],
    ) -> Result<usize, String> {
        let mut seen = std::collections::HashSet::new();
        for (name, codepoint) in assignments {
            if !self.glyphs.contains_key(name) {
                return Err(format!("グリフ '{}' がありません", name));
            }
            if char::from_u32(*codepoint).is_none() {
                return Err(format!("グリフ '{}' のUnicodeが不正です", name));
            }
            if !seen.insert(*codepoint) {
                return Err(format!("Unicode U+{:04X} が重複しています", codepoint));
            }
        }
        Ok(self.set_unicode_assignments(assignments))
    }
}
