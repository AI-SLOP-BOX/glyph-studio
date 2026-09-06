use super::*;

impl Contour {
    /// Moves a selection of points without moving any point twice. Smooth
    /// handles follow their on-curve point only when they are not themselves
    /// part of the selection.
    pub fn translate_points(&mut self, indices: &[usize], dx: f64, dy: f64) {
        let selected: std::collections::HashSet<usize> = indices
            .iter()
            .copied()
            .filter(|&index| index < self.points.len())
            .collect();
        let mut move_indices = selected.clone();
        let len = self.points.len();
        for &index in &selected {
            if self.points[index].is_on_curve() && self.points[index].smooth {
                for neighbor in [(index + len - 1) % len, (index + 1) % len] {
                    if !self.points[neighbor].is_on_curve() && !selected.contains(&neighbor) {
                        move_indices.insert(neighbor);
                    }
                }
            }
        }
        for index in move_indices {
            self.points[index].x += dx;
            self.points[index].y += dy;
        }
        self.repair_smooth_handles();
    }
}
