use super::*;

impl GlyphData {
    /// Keeps the intersection of adjacent contours in every layer.
    pub fn intersection_contours_all_layers(&mut self, index: usize) -> Result<(), String> {
        let next = index
            .checked_add(1)
            .ok_or_else(|| "輪郭番号が大きすぎます".to_string())?;
        let calculate = |contours: &[Contour]| -> Result<Vec<Contour>, String> {
            let first = contours
                .get(index)
                .ok_or_else(|| "選択輪郭が存在しません".to_string())?;
            let second = contours
                .get(next)
                .ok_or_else(|| "交差対象の次の輪郭が存在しません".to_string())?;
            let mut result = contours.to_vec();
            let intersection = first.intersection(second)?;
            result.remove(next);
            result.remove(index);
            result.splice(index..index, intersection);
            Ok(result)
        };
        let authored = calculate(&self.contours)?;
        let layer_results: Vec<_> = self
            .layers
            .iter()
            .map(|(id, layer)| calculate(&layer.contours).map(|contours| (id.clone(), contours)))
            .collect::<Result<_, _>>()?;
        self.contours = authored;
        for (id, contours) in layer_results {
            if let Some(layer) = self.layers.get_mut(&id) {
                layer.contours = contours;
            }
        }
        Ok(())
    }
}
