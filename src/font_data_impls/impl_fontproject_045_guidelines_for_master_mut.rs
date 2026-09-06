use super::*;

impl FontProject {
    pub fn guidelines_for_master_mut(&mut self, master_id: &str) -> &mut Vec<Guideline> {
        if !self.guidelines_by_master.contains_key(master_id) {
            self.guidelines_by_master
                .insert(master_id.to_string(), self.guidelines.clone());
        }
        self.guidelines_by_master
            .get_mut(master_id)
            .expect("global guide entry inserted")
    }
}
