use super::*;

impl GlyphData {
    /// Cuts a contour at two nodes in the authored outline and every layer.
    pub fn cut_contour_all_layers(
        &mut self,
        contour_index: usize,
        first: usize,
        second: usize,
    ) -> Result<(), String> {
        let mut candidate = self.clone();
        let replace = |contours: &mut Vec<Contour>| -> Result<(), String> {
            let (first_contour, second_contour) = contours
                .get(contour_index)
                .and_then(|contour| contour.cut_at_nodes(first, second))
                .ok_or_else(|| "2点の間隔が短すぎるため輪郭を分割できません".to_string())?;
            contours.remove(contour_index);
            contours.insert(contour_index, second_contour);
            contours.insert(contour_index, first_contour);
            Ok(())
        };
        replace(&mut candidate.contours)?;
        for layer in candidate.layers.values_mut() {
            replace(&mut layer.contours)?;
        }
        *self = candidate;
        Ok(())
    }
}
