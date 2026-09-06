use super::*;

impl FontProject {
    #[allow(dead_code)]
    pub fn set_vertical_metrics(
        &mut self,
        name: &str,
        advance_height: f64,
        top_side_bearing: f64,
    ) -> Result<(), String> {
        if !self.glyphs.contains_key(name) {
            return Err(format!("グリフ '{}' がありません", name));
        }
        if !advance_height.is_finite() || advance_height < 0.0 || !top_side_bearing.is_finite() {
            return Err("縦メトリクスが不正です".into());
        }
        self.vertical_metrics.insert(
            name.to_string(),
            VerticalMetrics {
                advance_height,
                top_side_bearing,
            },
        );
        Ok(())
    }
}
