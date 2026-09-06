use super::*;

impl FontProject {
    pub fn vertical_metrics_for_glyph_in_master(
        &self,
        name: &str,
        master_id: &str,
    ) -> VerticalMetrics {
        self.vertical_metrics_by_master
            .get(master_id)
            .and_then(|metrics| metrics.get(name).copied())
            .unwrap_or_else(|| self.vertical_metrics_for_glyph(name))
    }
}
