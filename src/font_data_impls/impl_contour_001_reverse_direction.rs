use super::*;

impl Contour {
    /// Reverses the winding direction while preserving the authored nodes.
    pub fn reverse_direction(&mut self) {
        self.points.reverse();
    }
}
