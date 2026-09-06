use super::*;

impl FontProject {
    pub fn clear_master_metrics(&mut self, master_id: &str) -> bool {
        self.metrics_by_master.remove(master_id).is_some()
    }
}
