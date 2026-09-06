use super::*;

impl FontProject {
    /// グリフの実効カーニング値と、その値を提供している保存キーを返す。
    pub fn kerning_source_for_glyphs(
        &self,
        left: &str,
        right: &str,
    ) -> Option<((String, String), f64)> {
        if let Some(value) = self.kerning.get(&(left.to_string(), right.to_string())) {
            return Some(((left.to_string(), right.to_string()), *value));
        }
        let left_group = self.glyphs.get(left)?.left_kerning_group.trim();
        let right_group = self.glyphs.get(right)?.right_kerning_group.trim();
        if left_group.is_empty() || right_group.is_empty() {
            return None;
        }
        self.kerning
            .iter()
            .filter_map(|((pair_left, pair_right), value)| {
                let pair_left_group = self.glyphs.get(pair_left)?.left_kerning_group.trim();
                let pair_right_group = self.glyphs.get(pair_right)?.right_kerning_group.trim();
                (pair_left_group == left_group && pair_right_group == right_group)
                    .then_some(((pair_left.clone(), pair_right.clone()), *value))
            })
            .min_by(|(a, _), (b, _)| a.cmp(b))
    }
}
