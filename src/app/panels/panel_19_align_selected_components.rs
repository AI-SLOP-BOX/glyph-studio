use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn align_selected_components(&mut self, horizontal: bool) {
        let Some(name) = self.current_glyph.clone() else {
            return;
        };
        let Some(glyph) = self.project.glyphs.get(&name) else {
            return;
        };
        let selected = self.selected_component_indices();
        let centers: Vec<(usize, f64, f64)> = selected
            .into_iter()
            .filter_map(|index| {
                let component = glyph.components.get(index)?;
                let (x, y) = Self::component_visual_center(&self.project, component)?;
                Some((index, x, y))
            })
            .collect();
        if centers.len() < 2 {
            return;
        }
        let target = centers
            .iter()
            .map(|(_, x, y)| if horizontal { *y } else { *x })
            .sum::<f64>()
            / centers.len() as f64;
        let deltas: Vec<(usize, f64, f64)> = centers
            .into_iter()
            .map(|(index, x, y)| {
                if horizontal {
                    (index, 0.0, target - y)
                } else {
                    (index, target - x, 0.0)
                }
            })
            .collect();
        self.translate_selected_components_by(&deltas);
        self.status_message = "選択部品を整列しました".to_string();
    }
}
