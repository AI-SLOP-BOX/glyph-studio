use super::*;

impl GlyphData {
    /// Applies a scale/rotation to the same nodes in every master layer.
    pub fn transform_nodes_all_layers(
        &mut self,
        nodes: &[(usize, usize)],
        scale: f64,
        angle_radians: f64,
    ) -> Result<(), String> {
        if !scale.is_finite() || !angle_radians.is_finite() || scale == 0.0 {
            return Err("変形値が不正です".into());
        }
        let valid = |contours: &[Contour]| {
            nodes.iter().all(|&(contour_index, point_index)| {
                contours
                    .get(contour_index)
                    .and_then(|contour| contour.points.get(point_index))
                    .is_some()
            })
        };
        if !valid(&self.contours) || self.layers.values().any(|layer| !valid(&layer.contours)) {
            return Err("マスター間で選択ノードの構造が一致しません".into());
        }
        let mut candidate = self.clone();
        let apply = |contours: &mut [Contour]| {
            let (cx, cy) = nodes.iter().fold((0.0, 0.0), |(x, y), &(ci, pi)| {
                let point = contours[ci].points[pi];
                (x + point.x, y + point.y)
            });
            let center = (cx / nodes.len() as f64, cy / nodes.len() as f64);
            let (sin, cos) = angle_radians.sin_cos();
            for &(ci, pi) in nodes {
                let point = &mut contours[ci].points[pi];
                let x = (point.x - center.0) * scale;
                let y = (point.y - center.1) * scale;
                point.x = center.0 + x * cos - y * sin;
                point.y = center.1 + x * sin + y * cos;
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
