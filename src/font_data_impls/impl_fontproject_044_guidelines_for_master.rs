use super::*;

impl FontProject {
    pub fn guidelines_for_master(&self, master_id: &str) -> &[Guideline] {
        self.guidelines_by_master
            .get(master_id)
            .map(Vec::as_slice)
            .unwrap_or(&self.guidelines)
    }
}
