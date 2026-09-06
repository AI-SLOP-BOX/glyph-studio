use super::*;

impl GlyphData {
    /// Toggles on/off-curve state for matching nodes in every layer.
    pub fn toggle_curve_nodes_all_layers(
        &mut self,
        nodes: &[(usize, usize)],
    ) -> Result<(), String> {
        let toggle = |contours: &[Contour]| -> Result<Vec<Contour>, String> {
            let mut result = contours.to_vec();
            for &(contour_index, point_index) in nodes {
                if let Some(point) = result
                    .get_mut(contour_index)
                    .and_then(|contour| contour.points.get_mut(point_index))
                {
                    point.point_type = match point.point_type {
                        PointType::OnCurve => PointType::OffCurve,
                        PointType::OffCurve => PointType::OnCurve,
                    };
                    if point.is_on_curve() {
                        point.smooth = false;
                    }
                }
            }
            for &contour_index in nodes.iter().map(|(contour_index, _)| contour_index) {
                let Some(contour) = result.get(contour_index) else {
                    continue;
                };
                if contour
                    .points
                    .iter()
                    .filter(|point| point.is_on_curve())
                    .count()
                    < 3
                {
                    return Err("オンカーブ点が3つ未満になるため変更できません".to_string());
                }
            }
            Ok(result)
        };
        let authored = toggle(&self.contours)?;
        let layers = self
            .layers
            .iter()
            .map(|(id, layer)| toggle(&layer.contours).map(|contours| (id.clone(), contours)))
            .collect::<Result<Vec<_>, _>>()?;
        self.contours = authored;
        for (id, contours) in layers {
            if let Some(layer) = self.layers.get_mut(&id) {
                layer.contours = contours;
            }
        }
        Ok(())
    }
}
