use super::*;

impl FontProject {
    /// 複数グリフの左右余白を検証してからまとめて設定する。
    pub fn set_side_bearings_batch<I, N>(&mut self, rows: I) -> Result<usize, String>
    where
        I: IntoIterator<Item = (N, f64, f64)>,
        N: AsRef<str>,
    {
        let rows: Vec<(String, f64, f64)> = rows
            .into_iter()
            .map(|(name, left, right)| (name.as_ref().to_string(), left, right))
            .collect();
        for (name, left, right) in &rows {
            if !self.glyphs.contains_key(name) {
                return Err(format!("グリフ '{}' がありません", name));
            }
            if !left.is_finite() || !right.is_finite() || *left < 0.0 || *right < 0.0 {
                return Err(format!("グリフ '{}' の余白が不正です", name));
            }
        }
        let count = rows.len();
        for (name, left, right) in rows {
            self.set_side_bearings(&[name], left, right);
        }
        Ok(count)
    }
}
