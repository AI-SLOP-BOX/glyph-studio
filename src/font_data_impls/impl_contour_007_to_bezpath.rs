use super::*;

impl Contour {
    pub fn to_bezpath(&self) -> BezPath {
        let mut path = BezPath::new();
        if self.points.is_empty() {
            return path;
        }

        let points = &self.points;
        let start_idx = points.iter().position(|p| p.is_on_curve()).unwrap_or(0);
        let first = points[start_idx];

        path.move_to(Point::new(first.x, first.y));

        let mut i = (start_idx + 1) % points.len();
        while i != start_idx {
            let p = points[i];
            if p.is_on_curve() {
                path.line_to(Point::new(p.x, p.y));
            } else {
                let mut off_curve = vec![p];
                let next_i = (i + 1) % points.len();
                let next = points[next_i];

                if !next.is_on_curve() {
                    off_curve.push(next);
                    let after_next = points[(next_i + 1) % points.len()];
                    if off_curve.len() == 2 {
                        path.curve_to(
                            Point::new(off_curve[0].x, off_curve[0].y),
                            Point::new(off_curve[1].x, off_curve[1].y),
                            Point::new(after_next.x, after_next.y),
                        );
                        i = (next_i + 1) % points.len();
                        continue;
                    }
                }

                path.quad_to(Point::new(p.x, p.y), Point::new(next.x, next.y));
                i = next_i;
            }
            i = (i + 1) % points.len();
        }

        path.close_path();
        path
    }
}
