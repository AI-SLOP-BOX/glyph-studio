use super::*;

impl GlyphData {
    /// Reverses one contour in the authored geometry and every master layer.
    pub fn reverse_contour_all_layers(&mut self, index: usize) -> Result<(), String> {
        let reverse = |contours: &[Contour]| -> Result<Vec<Contour>, String> {
            let mut result = contours.to_vec();
            let contour = result
                .get_mut(index)
                .ok_or_else(|| "選択輪郭が存在しません".to_string())?;
            contour.reverse_direction();
            Ok(result)
        };
        let authored = reverse(&self.contours)?;
        let layers = self
            .layers
            .iter()
            .map(|(id, layer)| reverse(&layer.contours).map(|contours| (id.clone(), contours)))
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
