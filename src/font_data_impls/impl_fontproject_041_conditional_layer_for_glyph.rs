use super::*;

impl FontProject {
    pub fn conditional_layer_for_glyph(
        &self,
        glyph_name: &str,
        axis_values: &HashMap<String, f64>,
    ) -> Option<&ConditionalLayer> {
        self.conditional_layers
            .get(glyph_name)?
            .iter()
            .filter(|layer| {
                layer.conditions.iter().all(|(tag, range)| {
                    let value = axis_values.get(tag).or_else(|| {
                        axis_values
                            .iter()
                            .find(|(axis, _)| axis.eq_ignore_ascii_case(tag))
                            .map(|(_, value)| value)
                    });
                    let Some(value) = value else {
                        return false;
                    };
                    range.min.is_none_or(|min| *value >= min)
                        && range.max.is_none_or(|max| *value <= max)
                })
            })
            .max_by(|left, right| {
                let condition_order = left.conditions.len().cmp(&right.conditions.len());
                if condition_order != std::cmp::Ordering::Equal {
                    return condition_order;
                }
                let span = |layer: &ConditionalLayer| {
                    layer.conditions.values().fold(0.0, |total, range| {
                        total
                            + match (range.min, range.max) {
                                (Some(min), Some(max)) => (max - min).max(0.0),
                                _ => f64::INFINITY,
                            }
                    })
                };
                span(right)
                    .partial_cmp(&span(left))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }
}
