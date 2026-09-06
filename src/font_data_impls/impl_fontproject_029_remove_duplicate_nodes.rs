use super::*;

impl FontProject {
    /// Removes consecutive duplicate nodes from selected glyphs and all layers.
    /// Contours that would become invalid are left untouched.
    pub fn remove_duplicate_nodes(&mut self, names: &[String]) -> usize {
        let mut removed = 0;
        let mut clean = |contours: &mut Vec<Contour>| {
            for contour in contours {
                if contour.points.len() < 3 {
                    continue;
                }
                let original_points = std::mem::take(&mut contour.points);
                let original = original_points.len();
                let mut points = Vec::with_capacity(original);
                for point in original_points.iter().copied() {
                    if points
                        .last()
                        .is_some_and(|last: &ContourPoint| last.x == point.x && last.y == point.y)
                    {
                        continue;
                    }
                    points.push(point);
                }
                if points.len() > 1
                    && points.first().is_some_and(|first| {
                        points
                            .last()
                            .is_some_and(|last| first.x == last.x && first.y == last.y)
                    })
                {
                    points.pop();
                }
                if points.len() >= 3 {
                    removed += original - points.len();
                    contour.points = points;
                } else {
                    contour.points = original_points;
                }
            }
        };
        for name in names {
            if let Some(glyph) = self.glyphs.get_mut(name) {
                clean(&mut glyph.contours);
                for layer in glyph.layers.values_mut() {
                    clean(&mut layer.contours);
                }
            }
        }
        removed
    }
}
