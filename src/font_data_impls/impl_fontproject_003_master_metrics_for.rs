use super::*;

impl FontProject {
    pub fn master_metrics_for(&self, master_id: &str) -> MasterMetrics {
        self.metrics_by_master
            .get(master_id)
            .copied()
            .unwrap_or(MasterMetrics {
                ascender: self.metadata.ascender,
                descender: self.metadata.descender,
                line_gap: self.metadata.line_gap,
            })
    }
}
