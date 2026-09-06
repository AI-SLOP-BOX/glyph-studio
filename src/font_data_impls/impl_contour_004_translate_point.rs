use super::*;

impl Contour {
    pub fn translate_point(&mut self, index: usize, dx: f64, dy: f64) {
        let len = self.points.len();
        if index >= len {
            return;
        }
        let was_smooth = self.points[index].is_on_curve() && self.points[index].smooth;
        self.points[index].x += dx;
        self.points[index].y += dy;
        if was_smooth {
            for neighbor in [(index + len - 1) % len, (index + 1) % len] {
                if !self.points[neighbor].is_on_curve() {
                    self.points[neighbor].x += dx;
                    self.points[neighbor].y += dy;
                }
            }
        } else if !self.points[index].is_on_curve() {
            self.align_opposite_smooth_handle(index);
        }
    }
}
