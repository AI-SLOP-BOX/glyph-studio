use super::*;

impl GlyphData {
    /// Applies smooth/corner state to the same node positions in all layers.
    pub fn set_smooth_nodes_all_layers(
        &mut self,
        nodes: &[(usize, usize)],
        smooth: bool,
    ) -> Result<(), String> {
        let valid = |contours: &[Contour]| {
            nodes.iter().all(|&(contour_index, point_index)| {
                contours
                    .get(contour_index)
                    .and_then(|contour| contour.points.get(point_index))
                    .is_some()
            })
        };
        if !valid(&self.contours) || self.layers.values().any(|layer| !valid(&layer.contours)) {
            return Err("マスター間で選択ノードの構造が一致しません".to_string());
        }
        let apply = |contours: &mut [Contour]| {
            for &(contour_index, point_index) in nodes {
                if let Some(contour) = contours.get_mut(contour_index) {
                    if smooth {
                        contour.set_smooth(point_index, true);
                    } else if let Some(point) = contour.points.get_mut(point_index) {
                        point.smooth = false;
                    }
                }
            }
        };
        apply(&mut self.contours);
        for layer in self.layers.values_mut() {
            apply(&mut layer.contours);
        }
        Ok(())
    }
}
