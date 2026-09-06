use super::*;

impl GlyphData {
    /// Unions every authored contour into one Boolean result in every layer.
    pub fn union_all_contours_all_layers(&mut self) -> Result<(), String> {
        let calculate = |contours: &[Contour]| -> Result<Vec<Contour>, String> {
            if contours.len() < 2 {
                return Err("統合する輪郭が2つ以上必要です".to_string());
            }
            let mut result = vec![contours[0].clone()];
            for contour in &contours[1..] {
                let mut merged = Vec::new();
                for current in result {
                    merged.extend(current.union(contour)?);
                }
                result = merged;
            }
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
