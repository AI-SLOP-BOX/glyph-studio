use super::*;

impl FontProject {
    pub fn set_kerning_pair_for_master(
        &mut self,
        master_id: &str,
        left: impl Into<String>,
        right: impl Into<String>,
        value: f64,
    ) -> Result<(), String> {
        if !self.masters.iter().any(|master| master.id == master_id) {
            return Err(format!("マスター '{}' がありません", master_id));
        }
        if !value.is_finite() {
            return Err("カーニング値が不正です".into());
        }
        let left = left.into();
        let right = right.into();
        if left.trim().is_empty() || right.trim().is_empty() {
            return Err("カーニングの左右キーを空にできません".into());
        }
        let fallback = self.kerning.clone();
        let pairs = self
            .kerning_by_master
            .entry(master_id.to_string())
            .or_insert(fallback);
        pairs.insert((left.clone(), right.clone()), value);
        if master_id == self.default_master_id {
            self.kerning.insert((left, right), value);
        }
        Ok(())
    }
}
