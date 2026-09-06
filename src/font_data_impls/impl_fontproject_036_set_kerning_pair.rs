use super::*;

impl FontProject {
    /// グリフまたはグループのカーニングペアを設定する。
    pub fn set_kerning_pair(
        &mut self,
        left: impl Into<String>,
        right: impl Into<String>,
        value: f64,
    ) -> Result<(), String> {
        if !value.is_finite() {
            return Err("カーニング値が不正です".into());
        }
        let left = left.into();
        let right = right.into();
        if left.trim().is_empty() || right.trim().is_empty() {
            return Err("カーニングの左右キーを空にできません".into());
        }
        self.kerning.insert((left, right), value);
        Ok(())
    }
}
