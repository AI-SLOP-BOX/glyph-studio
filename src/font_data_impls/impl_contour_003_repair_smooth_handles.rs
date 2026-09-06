use super::*;

impl Contour {
    /// Re-aligns every smooth node after a bulk transform.
    pub fn repair_smooth_handles(&mut self) {
        let smooth_nodes: Vec<usize> = self
            .points
            .iter()
            .enumerate()
            .filter_map(|(index, point)| (point.is_on_curve() && point.smooth).then_some(index))
            .collect();
        for index in smooth_nodes {
            self.set_smooth(index, true);
        }
    }
}
