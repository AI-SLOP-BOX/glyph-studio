use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn select_relative_master(&mut self, delta: isize) {
        let master_ids: Vec<String> = self
            .project
            .masters
            .iter()
            .map(|master| master.id.clone())
            .collect();
        if master_ids.is_empty() {
            return;
        }
        let current = master_ids
            .iter()
            .position(|id| id == &self.current_master_id)
            .unwrap_or(0) as isize;
        let next = (current + delta).clamp(0, master_ids.len() as isize - 1) as usize;
        if master_ids[next] != self.current_master_id {
            let previous = self.current_master_id.clone();
            self.project.switch_master(&previous, &master_ids[next]);
            self.current_master_id = master_ids[next].clone();
            self.selected_guideline = None;
            self.guideline_drag = None;
            self.project.sync_active_layer(&self.current_master_id);
            self.status_message = format!(
                "マスター: {}",
                self.project
                    .masters
                    .iter()
                    .find(|master| master.id == self.current_master_id)
                    .map(|master| master.name.as_str())
                    .unwrap_or(self.current_master_id.as_str())
            );
        }
    }
}
