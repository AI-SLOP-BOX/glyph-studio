use super::*;

impl GlyphData {
    /// Reflects the same nodes around their selection bounds in every layer.
    pub fn reflect_nodes_all_layers(
        &mut self,
        nodes: &[(usize, usize)],
        horizontal: bool,
    ) -> Result<(), String> {
        let valid = |contours: &[Contour]| {
            nodes.iter().all(|&(contour_index, point_index)| {
                contours
                    .get(contour_index)
                    .and_then(|contour| contour.points.get(point_index))
                    .is_some()
            })
        };
        if nodes.is_empty()
            || !valid(&self.contours)
            || self.layers.values().any(|layer| !valid(&layer.contours))
        {
            return Err("マスター間で選択ノードの構造が一致しません".into());
        }
        let mut candidate = self.clone();
        let apply = |contours: &mut [Contour]| {
            let points = nodes.iter().map(|&(ci, pi)| contours[ci].points[pi]);
            let (min_x, max_x, min_y, max_y) = points.fold(
                (
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                ),
                |(min_x, max_x, min_y, max_y), point| {
                    (
                        min_x.min(point.x),
                        max_x.max(point.x),
                        min_y.min(point.y),
                        max_y.max(point.y),
                    )
                },
            );
            let center = ((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
            for &(ci, pi) in nodes {
                let point = &mut contours[ci].points[pi];
                if horizontal {
                    point.x = center.0 - (point.x - center.0);
                } else {
                    point.y = center.1 - (point.y - center.1);
                }
            }
            for contour in contours {
                contour.repair_smooth_handles();
            }
        };
        apply(&mut candidate.contours);
        for layer in candidate.layers.values_mut() {
            apply(&mut layer.contours);
        }
        *self = candidate;
        Ok(())
    }
}
