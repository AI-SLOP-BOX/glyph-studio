use super::*;

impl Contour {
    /// Splits a line, quadratic, or cubic segment whose start point is
    /// `start_index`. The segment must not wrap around the point vector.
    /// Returns the inserted on-curve point index.
    pub fn split_segment(&mut self, start_index: usize, t: f64) -> Option<usize> {
        if self.points.len() < 2 || start_index >= self.points.len() {
            return None;
        }
        let start = self.points[start_index];
        if !start.is_on_curve() {
            return None;
        }
        let mut end = start_index + 1;
        while end < self.points.len() && !self.points[end].is_on_curve() {
            end += 1;
        }
        if end >= self.points.len() {
            // The segment wraps around the cyclic contour. Rotate it so the
            // existing non-wrapping implementation can preserve its controls.
            if start_index == 0 {
                return None;
            }
            let rotation = start_index;
            self.points.rotate_left(rotation);
            let inserted = self.split_segment(0, t);
            let new_len = self.points.len();
            self.points.rotate_right(rotation % new_len);
            return inserted.map(|index| (index + rotation) % new_len);
        }
        if end - start_index > 3 {
            return None;
        }
        let t = t.clamp(0.0, 1.0);
        let lerp = |a: ContourPoint, b: ContourPoint| {
            ContourPoint::on_curve(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
        };
        let mut replacement = Vec::new();
        match end - start_index {
            1 => replacement.push(lerp(start, self.points[end])),
            2 => {
                let c = self.points[start_index + 1];
                let mut left = lerp(start, c);
                let mut right = lerp(c, self.points[end]);
                left.point_type = PointType::OffCurve;
                right.point_type = PointType::OffCurve;
                let q = lerp(left, right);
                replacement.extend([left, q, right]);
            }
            3 => {
                let c1 = self.points[start_index + 1];
                let c2 = self.points[start_index + 2];
                let mut a = lerp(start, c1);
                let mut b = lerp(c1, c2);
                let mut c = lerp(c2, self.points[end]);
                let mut d = lerp(a, b);
                let mut e = lerp(b, c);
                for point in [&mut a, &mut b, &mut c, &mut d, &mut e] {
                    point.point_type = PointType::OffCurve;
                }
                let q = lerp(d, e);
                replacement.extend([a, d, q, e, c]);
            }
            _ => return None,
        }
        self.points.splice((start_index + 1)..end, replacement);
        Some(start_index + 1 + if end - start_index == 1 { 0 } else { 1 })
    }
}
