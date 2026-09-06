use super::*;

impl FontProject {
    /// 指定したカーニングペアを削除し、削除できたかを返す。
    pub fn remove_kerning_pair(&mut self, left: &str, right: &str) -> bool {
        self.kerning
            .remove(&(left.to_string(), right.to_string()))
            .is_some()
    }
}
