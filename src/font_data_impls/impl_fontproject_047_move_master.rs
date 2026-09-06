use super::*;

impl FontProject {
    /// Moves a master in the project order without changing its identity or
    /// any glyph layer data. The order is also used when presenting and
    /// exporting master-specific results.
    pub fn move_master(&mut self, master_id: &str, delta: isize) -> bool {
        let Some(index) = self
            .masters
            .iter()
            .position(|master| master.id == master_id)
        else {
            return false;
        };
        let target = index as isize + delta;
        if !(0..self.masters.len() as isize).contains(&target) || target == index as isize {
            return false;
        }
        self.masters.swap(index, target as usize);
        true
    }
}
