use super::*;

impl FontProject {
    pub fn set_master_metrics(
        &mut self,
        master_id: &str,
        metrics: MasterMetrics,
    ) -> Result<(), String> {
        if !self.masters.iter().any(|master| master.id == master_id) {
            return Err(format!("マスター '{}' がありません", master_id));
        }
        if !metrics.ascender.is_finite()
            || !metrics.descender.is_finite()
            || !metrics.line_gap.is_finite()
        {
            return Err("マスターメトリクスは有限値で指定してください".into());
        }
        self.metrics_by_master
            .insert(master_id.to_string(), metrics);
        Ok(())
    }
}
