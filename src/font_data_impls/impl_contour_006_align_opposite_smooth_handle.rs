use super::*;

impl Contour {
    pub(crate) fn align_opposite_smooth_handle(&mut self, handle_index: usize) {
        let len = self.points.len();
        if len < 3 || handle_index >= len {
            return;
        }
        let previous = (handle_index + len - 1) % len;
        let next = (handle_index + 1) % len;
        let (anchor_index, opposite_index) =
            if self.points[previous].is_on_curve() && self.points[previous].smooth {
                let opposite = (previous + len - 1) % len;
                (previous, opposite)
            } else if self.points[next].is_on_curve() && self.points[next].smooth {
                let opposite = (next + 1) % len;
                (next, opposite)
            } else {
                return;
            };
        if self.points[opposite_index].is_on_curve() {
            return;
        }
        let anchor = self.points[anchor_index];
        let handle = self.points[handle_index];
        let dx = handle.x - anchor.x;
        let dy = handle.y - anchor.y;
        let length = (dx * dx + dy * dy).sqrt();
        if length <= f64::EPSILON {
            return;
        }
        let opposite_length = {
            let point = self.points[opposite_index];
            ((point.x - anchor.x).powi(2) + (point.y - anchor.y).powi(2)).sqrt()
        };
        if opposite_length <= f64::EPSILON {
            return;
        }
        let ux = dx / length;
        let uy = dy / length;
        self.points[opposite_index].x = anchor.x - ux * opposite_length;
        self.points[opposite_index].y = anchor.y - uy * opposite_length;
    }
}
