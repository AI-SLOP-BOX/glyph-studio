use super::*;

impl GlyphData {
    /// Reflects one component in every master layer.
    pub fn reflect_component_all_layers(
        &mut self,
        component_index: usize,
        horizontal: bool,
    ) -> Result<(), String> {
        let base = self
            .components
            .get(component_index)
            .map(|component| component.base.as_str())
            .ok_or_else(|| "コンポーネントがありません".to_string())?;
        if self.layers.values().any(|layer| {
            layer
                .components
                .get(component_index)
                .is_none_or(|component| component.base != base)
        }) {
            return Err("マスター間でコンポーネント構造が一致しません".into());
        }
        let mut candidate = self.clone();
        let apply = |component: &mut GlyphComponent| {
            if horizontal {
                component.x_scale = -component.x_scale;
                component.xy_scale = -component.xy_scale;
            } else {
                component.yx_scale = -component.yx_scale;
                component.y_scale = -component.y_scale;
            }
        };
        apply(&mut candidate.components[component_index]);
        for layer in candidate.layers.values_mut() {
            apply(&mut layer.components[component_index]);
        }
        *self = candidate;
        Ok(())
    }
}
