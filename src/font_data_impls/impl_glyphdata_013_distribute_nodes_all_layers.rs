use super::*;

impl GlyphData {
    /// Distributes selected nodes evenly on one axis in every master layer.
    pub fn distribute_nodes_all_layers(
        &mut self,
        nodes: &[(usize, usize)],
        horizontal: bool,
    ) -> Result<(), String> {
        if nodes.len() < 3 {
            return Err("分布には3つ以上のノードが必要です".into());
        }
        self.map_nodes_all_layers(nodes, |points| {
            let mut ordered: Vec<(f64, usize)> = points
                .iter()
                .enumerate()
                .map(|(index, point)| (if horizontal { point.x } else { point.y }, index))
                .collect();
            ordered.sort_by(|a, b| a.0.total_cmp(&b.0));
            let first = ordered.first().map(|item| item.0).unwrap_or_default();
            let last = ordered.last().map(|item| item.0).unwrap_or(first);
            let step = (last - first) / (ordered.len() - 1) as f64;
            for (index, (_, point_index)) in ordered.into_iter().enumerate() {
                if horizontal {
                    points[point_index].x = first + step * index as f64;
                } else {
                    points[point_index].y = first + step * index as f64;
                }
            }
        })
    }
}
