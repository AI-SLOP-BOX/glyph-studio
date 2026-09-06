use super::*;

impl FontProject {
    pub fn set_vertical_metrics_for_master(
        &mut self,
        name: &str,
        master_id: &str,
        advance_height: f64,
        top_side_bearing: f64,
    ) -> Result<(), String> {
        if !self.glyphs.contains_key(name) {
            return Err(format!("グリフ '{}' がありません", name));
        }
        if !advance_height.is_finite() || advance_height < 0.0 {
            return Err("縦アドバンスは0以上の有限値で指定してください".into());
        }
        if !top_side_bearing.is_finite() {
            return Err("縦TSBは有限値で指定してください".into());
        }
        self.vertical_metrics_by_master
            .entry(master_id.to_string())
            .or_default()
            .insert(
                name.to_string(),
                VerticalMetrics {
                    advance_height,
                    top_side_bearing,
                },
            );
        Ok(())
    }
}
