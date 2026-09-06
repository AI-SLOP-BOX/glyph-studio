use super::*;

impl GlyphData {
    /// Applies a scale/rotation to one component in every master layer.
    pub fn transform_component_all_layers(
        &mut self,
        component_index: usize,
        scale: f64,
        angle_radians: f64,
    ) -> Result<(), String> {
        if !scale.is_finite() || !angle_radians.is_finite() || scale == 0.0 {
            return Err("変形値が不正です".into());
        }
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
        let (sin, cos) = angle_radians.sin_cos();
        let apply = |component: &mut GlyphComponent| {
            let a = component.x_scale;
            let b = component.xy_scale;
            let c = component.yx_scale;
            let d = component.y_scale;
            component.x_scale = scale * (cos * a - sin * b);
            component.xy_scale = scale * (sin * a + cos * b);
            component.yx_scale = scale * (cos * c - sin * d);
            component.y_scale = scale * (sin * c + cos * d);
        };
        apply(&mut candidate.components[component_index]);
        for layer in candidate.layers.values_mut() {
            apply(&mut layer.components[component_index]);
        }
        *self = candidate;
        Ok(())
    }
}
