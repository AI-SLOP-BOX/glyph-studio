use super::*;

impl FontProject {
    /// 複数グリフの字幅を検証してからまとめて設定する。
    pub fn set_widths_batch<I, N>(&mut self, rows: I) -> Result<usize, String>
    where
        I: IntoIterator<Item = (N, f64)>,
        N: AsRef<str>,
    {
        let rows: Vec<(String, f64)> = rows
            .into_iter()
            .map(|(name, width)| (name.as_ref().to_string(), width))
            .collect();
        for (name, width) in &rows {
            if !self.glyphs.contains_key(name) {
                return Err(format!("グリフ '{}' がありません", name));
            }
            if !width.is_finite() || *width < 0.0 {
                return Err(format!("グリフ '{}' の字幅が不正です", name));
            }
        }
        let count = rows.len();
        for (name, width) in rows {
            self.set_width_for_glyphs(&[name], width);
        }
        Ok(count)
    }
}
