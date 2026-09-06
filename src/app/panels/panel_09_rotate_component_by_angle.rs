use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn rotate_component_by_angle(
        project: &FontProject,
        original: &GlyphComponent,
        angle: f64,
    ) -> Option<GlyphComponent> {
        let (min_x, min_y, max_x, max_y) = project.outline_bounds_for_glyph(&original.base)?;
        let center_local = ((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
        let center = (
            original.x_scale * center_local.0
                + original.yx_scale * center_local.1
                + original.x_offset,
            original.xy_scale * center_local.0
                + original.y_scale * center_local.1
                + original.y_offset,
        );
        let (sin, cos) = angle.sin_cos();
        let mut rotated = original.clone();
        rotated.x_scale = cos * original.x_scale - sin * original.xy_scale;
        rotated.xy_scale = sin * original.x_scale + cos * original.xy_scale;
        rotated.yx_scale = cos * original.yx_scale - sin * original.y_scale;
        rotated.y_scale = sin * original.yx_scale + cos * original.y_scale;
        rotated.x_offset =
            center.0 - rotated.x_scale * center_local.0 - rotated.yx_scale * center_local.1;
        rotated.y_offset =
            center.1 - rotated.xy_scale * center_local.0 - rotated.y_scale * center_local.1;
        Some(rotated)
    }
}
