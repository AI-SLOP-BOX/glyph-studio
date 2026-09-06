use super::*;

impl GlyphData {
    /// Removes one contour from authored geometry and every master layer.
    pub fn remove_contour_all_layers(&mut self, index: usize) -> Result<(), String> {
        if index >= self.contours.len()
            || self
                .layers
                .values()
                .any(|layer| index >= layer.contours.len())
        {
            return Err("マスター間で削除対象の輪郭構造が一致しません".to_string());
        }
        self.contours.remove(index);
        for layer in self.layers.values_mut() {
            layer.contours.remove(index);
        }
        Ok(())
    }
}
