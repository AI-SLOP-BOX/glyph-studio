use super::*;

impl GlyphData {
    /// Removes selected nodes from the authored outline and every saved
    /// master layer atomically. Fragments left without enough authored data
    /// are discarded consistently in every layer.
    pub fn remove_nodes_all_layers(&mut self, nodes: &[(usize, usize)]) -> Result<(), String> {
        if nodes.is_empty() {
            return Ok(());
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
            return Err("マスター間で選択ノードの構造が一致しません".to_string());
        }
        let mut candidate = self.clone();
        let apply = |contours: &mut Vec<Contour>| {
            let mut grouped: std::collections::BTreeMap<usize, Vec<usize>> =
                std::collections::BTreeMap::new();
            for &(contour_index, point_index) in nodes {
                grouped.entry(contour_index).or_default().push(point_index);
            }
            for (contour_index, mut point_indices) in grouped {
                if let Some(contour) = contours.get_mut(contour_index) {
                    point_indices.sort_unstable();
                    point_indices.dedup();
                    for point_index in point_indices.into_iter().rev() {
                        if point_index < contour.points.len() {
                            contour.points.remove(point_index);
                        }
                    }
                }
            }
            contours.retain(|contour| {
                contour.points.len() >= 3 && contour.points.iter().any(|point| point.is_on_curve())
            });
        };
        apply(&mut candidate.contours);
        for layer in candidate.layers.values_mut() {
            apply(&mut layer.contours);
        }
        *self = candidate;
        Ok(())
    }
}
