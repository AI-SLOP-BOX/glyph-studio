use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn resize_component_from_handle(
        project: &FontProject,
        original: &GlyphComponent,
        handle: usize,
        target: (f64, f64),
    ) -> Option<GlyphComponent> {
        let (min_x, min_y, max_x, max_y) = project.outline_bounds_for_glyph(&original.base)?;
        let corners = [
            (min_x, min_y),
            (min_x, max_y),
            (max_x, max_y),
            (max_x, min_y),
        ];
        let handle = handle.min(3);
        let opposite = (handle + 2) % 4;
        let transform = |point: (f64, f64), component: &GlyphComponent| {
            (
                component.x_scale * point.0 + component.yx_scale * point.1 + component.x_offset,
                component.xy_scale * point.0 + component.y_scale * point.1 + component.y_offset,
            )
        };
        let fixed = transform(corners[opposite], original);
        let target_delta = (target.0 - fixed.0, target.1 - fixed.1);
        let local_delta = (
            corners[handle].0 - corners[opposite].0,
            corners[handle].1 - corners[opposite].1,
        );
        let x_axis = (
            original.x_scale * local_delta.0,
            original.xy_scale * local_delta.0,
        );
        let y_axis = (
            original.yx_scale * local_delta.1,
            original.y_scale * local_delta.1,
        );
        let determinant = x_axis.0 * y_axis.1 - x_axis.1 * y_axis.0;
        if determinant.abs() < 1.0e-9 {
            return None;
        }
        let scale_x = (target_delta.0 * y_axis.1 - target_delta.1 * y_axis.0) / determinant;
        let scale_y = (x_axis.0 * target_delta.1 - x_axis.1 * target_delta.0) / determinant;
        if !scale_x.is_finite() || !scale_y.is_finite() {
            return None;
        }
        let scale_x = scale_x.clamp(-100.0, 100.0);
        let scale_y = scale_y.clamp(-100.0, 100.0);
        let new_x_scale = original.x_scale * scale_x;
        let new_xy_scale = original.xy_scale * scale_x;
        let new_yx_scale = original.yx_scale * scale_y;
        let new_y_scale = original.y_scale * scale_y;
        let mut resized = original.clone();
        resized.x_scale = new_x_scale;
        resized.xy_scale = new_xy_scale;
        resized.yx_scale = new_yx_scale;
        resized.y_scale = new_y_scale;
        resized.x_offset = fixed.0
            - resized.x_scale * corners[opposite].0
            - resized.yx_scale * corners[opposite].1;
        resized.y_offset = fixed.1
            - resized.xy_scale * corners[opposite].0
            - resized.y_scale * corners[opposite].1;
        Some(resized)
    }
}
