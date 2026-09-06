use super::*;

impl Contour {
    /// Marks an on-curve node smooth and aligns its adjacent handles.
    pub fn set_smooth(&mut self, index: usize, smooth: bool) {
        let len = self.points.len();
        if index >= len || !self.points[index].is_on_curve() {
            return;
        }
        self.points[index].smooth = smooth;
        if !smooth || len < 3 {
            return;
        }
        let previous = (index + len - 1) % len;
        let next = (index + 1) % len;
        if self.points[previous].is_on_curve() || self.points[next].is_on_curve() {
            return;
        }
        let point = self.points[index];
        let before = self.points[previous];
        let after = self.points[next];
        let dx = after.x - before.x;
        let dy = after.y - before.y;
        let direction_length = (dx * dx + dy * dy).sqrt();
        if direction_length <= f64::EPSILON {
            return;
        }
        let before_length = ((before.x - point.x).powi(2) + (before.y - point.y).powi(2)).sqrt();
        let after_length = ((after.x - point.x).powi(2) + (after.y - point.y).powi(2)).sqrt();
        let ux = dx / direction_length;
        let uy = dy / direction_length;
        self.points[previous].x = point.x - ux * before_length;
        self.points[previous].y = point.y - uy * before_length;
        self.points[next].x = point.x + ux * after_length;
        self.points[next].y = point.y + uy * after_length;
    }
}
