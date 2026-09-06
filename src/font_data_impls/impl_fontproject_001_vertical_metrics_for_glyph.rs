use super::*;

impl FontProject {
    #[allow(dead_code)]
    pub fn vertical_metrics_for_glyph(&self, name: &str) -> VerticalMetrics {
        self.vertical_metrics
            .get(name)
            .copied()
            .unwrap_or(VerticalMetrics {
                advance_height: self.metadata.units_per_em,
                top_side_bearing: self.metadata.ascender,
            })
    }
}
