use super::*;

impl FontProject {
    /// 複数のカーニングペアを検証してからまとめて設定する。
    pub fn set_kerning_pairs<I, L, R>(&mut self, pairs: I) -> Result<usize, String>
    where
        I: IntoIterator<Item = (L, R, f64)>,
        L: AsRef<str>,
        R: AsRef<str>,
    {
        let pairs: Vec<(String, String, f64)> = pairs
            .into_iter()
            .map(|(left, right, value)| {
                (left.as_ref().to_string(), right.as_ref().to_string(), value)
            })
            .collect();
        for (left, right, value) in &pairs {
            if !value.is_finite() {
                return Err("カーニング値が不正です".into());
            }
            if left.trim().is_empty() || right.trim().is_empty() {
                return Err("カーニングの左右キーを空にできません".into());
            }
        }
        let count = pairs.len();
        for (left, right, value) in pairs {
            self.kerning.insert((left, right), value);
        }
        Ok(count)
    }
}
