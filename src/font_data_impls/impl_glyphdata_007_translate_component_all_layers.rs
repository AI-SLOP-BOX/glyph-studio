use super::*;

impl GlyphData {
    /// Moves one component in every master layer atomically.
    pub fn translate_component_all_layers(
        &mut self,
        component_index: usize,
        dx: f64,
        dy: f64,
    ) -> Result<(), String> {
        if !dx.is_finite() || !dy.is_finite() {
            return Err("移動量が不正です".into());
        }
        if component_index >= self.components.len()
            || self
                .layers
                .values()
                .any(|layer| component_index >= layer.components.len())
        {
            return Err("マスター間でコンポーネント構造が一致しません".into());
        }
        let mut candidate = self.clone();
        candidate.components[component_index].x_offset += dx;
        candidate.components[component_index].y_offset += dy;
        for layer in candidate.layers.values_mut() {
            layer.components[component_index].x_offset += dx;
            layer.components[component_index].y_offset += dy;
        }
        *self = candidate;
        Ok(())
    }
}
