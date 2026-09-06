use super::*;

impl GlyphData {
    pub(crate) fn map_nodes_all_layers<F>(
        &mut self,
        nodes: &[(usize, usize)],
        mut operation: F,
    ) -> Result<(), String>
    where
        F: FnMut(&mut Vec<ContourPoint>),
    {
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
        let apply = |contours: &mut [Contour], operation: &mut F| {
            let mut points: Vec<ContourPoint> = nodes
                .iter()
                .map(|&(contour_index, point_index)| contours[contour_index].points[point_index])
                .collect();
            operation(&mut points);
            for (&(contour_index, point_index), point) in nodes.iter().zip(points) {
                contours[contour_index].points[point_index] = point;
            }
            for contour in contours {
                contour.repair_smooth_handles();
            }
        };
        apply(&mut candidate.contours, &mut operation);
        for layer in candidate.layers.values_mut() {
            apply(&mut layer.contours, &mut operation);
        }
        *self = candidate;
        Ok(())
    }
}
