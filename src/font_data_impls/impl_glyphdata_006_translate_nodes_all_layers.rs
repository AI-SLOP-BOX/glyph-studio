use super::*;

impl GlyphData {
    /// Moves the same authored nodes in every master layer atomically.
    pub fn translate_nodes_all_layers(
        &mut self,
        nodes: &[(usize, usize)],
        dx: f64,
        dy: f64,
    ) -> Result<(), String> {
        if !dx.is_finite() || !dy.is_finite() {
            return Err("移動量が不正です".into());
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
            for (contour_index, contour) in contours.iter_mut().enumerate() {
                let indices: Vec<usize> = nodes
                    .iter()
                    .filter_map(|&(selected_contour, point_index)| {
                        (selected_contour == contour_index).then_some(point_index)
                    })
                    .collect();
                if !indices.is_empty() {
                    contour.translate_points(&indices, dx, dy);
                }
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
