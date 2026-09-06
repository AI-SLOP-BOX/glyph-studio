use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn distribute_selected_components(&mut self, horizontal: bool) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let Some(glyph) = self.project.glyphs.get(&name) else {
            return;
        };
        let selected = self.selected_component_indices();
        let mut centers: Vec<(usize, f64, f64)> = selected
            .into_iter()
            .filter_map(|index| {
                let component = glyph.components.get(index)?;
                let (x, y) = Self::component_visual_center(&self.project, component)?;
                Some((index, x, y))
            })
            .collect();
        if centers.len() < 3 {
            return;
        }
        centers.sort_by(|left, right| {
            let left_value = if horizontal { left.1 } else { left.2 };
            let right_value = if horizontal { right.1 } else { right.2 };
            left_value.total_cmp(&right_value)
        });
        let first = if horizontal {
            centers[0].1
        } else {
            centers[0].2
        };
        let last = if horizontal {
            centers.last().map(|item| item.1).unwrap_or(first)
        } else {
            centers.last().map(|item| item.2).unwrap_or(first)
        };
        let step = (last - first) / (centers.len() - 1) as f64;
        let deltas: Vec<(usize, f64, f64)> = centers
            .into_iter()
            .enumerate()
            .map(|(position, (index, x, y))| {
                let target = first + step * position as f64;
                if horizontal {
                    (index, target - x, 0.0)
                } else {
                    (index, 0.0, target - y)
                }
            })
            .collect();
        self.translate_selected_components_by(&deltas);
        self.status_message = "選択部品を分布しました".to_string();
    }
}
