use super::*;

impl Contour {
    /// Cuts a closed contour at two on-curve node indices.
    pub fn cut_at_nodes(&self, first: usize, second: usize) -> Option<(Self, Self)> {
        let len = self.points.len();
        if len < 4 || first >= len || second >= len || first == second {
            return None;
        }
        if !self.points[first].is_on_curve() || !self.points[second].is_on_curve() {
            return None;
        }
        let (start, end) = if first < second {
            (first, second)
        } else {
            (second, first)
        };
        if end - start < 2 || len - (end - start) < 2 {
            return None;
        }
        let first_path = self.points[start..=end].to_vec();
        let mut second_path = self.points[end..].to_vec();
        second_path.extend_from_slice(&self.points[..=start]);
        if first_path.len() < 3 || second_path.len() < 3 {
            return None;
        }
        Some((
            Self { points: first_path },
            Self {
                points: second_path,
            },
        ))
    }
}
