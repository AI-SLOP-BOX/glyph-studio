use super::*;

impl GlyphData {
    /// Inserts a node at the same segment position in the authored outline
    /// and every master layer, without leaving a partially edited glyph.
    pub fn split_segment_all_layers(
        &mut self,
        contour_index: usize,
        start_index: usize,
        factor: f64,
    ) -> Result<usize, String> {
        let mut candidate = self.clone();
        let inserted = candidate
            .contours
            .get_mut(contour_index)
            .and_then(|contour| contour.split_segment(start_index, factor))
            .ok_or_else(|| "輪郭を分割できません".to_string())?;
        for layer in candidate.layers.values_mut() {
            layer
                .contours
                .get_mut(contour_index)
                .and_then(|contour| contour.split_segment(start_index, factor))
                .ok_or_else(|| "マスター間で輪郭構造が一致しません".to_string())?;
        }
        *self = candidate;
        Ok(inserted)
    }
}
