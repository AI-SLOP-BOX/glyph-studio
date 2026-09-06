use super::*;

impl GlyphData {
    /// Aligns the selected nodes on one axis in every master layer.
    pub fn align_nodes_all_layers(
        &mut self,
        nodes: &[(usize, usize)],
        horizontal: bool,
    ) -> Result<(), String> {
        self.map_nodes_all_layers(nodes, |points| {
            let target = points
                .iter()
                .map(|point| if horizontal { point.y } else { point.x })
                .sum::<f64>()
                / points.len() as f64;
            for point in points {
                if horizontal {
                    point.y = target;
                } else {
                    point.x = target;
                }
            }
        })
    }
}
