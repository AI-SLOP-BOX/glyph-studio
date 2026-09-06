use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn rotate_component_from_handle(
        project: &FontProject,
        original: &GlyphComponent,
        start: (f64, f64),
        target: (f64, f64),
        snap_angle: bool,
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
        let start_angle = (start.1 - center.1).atan2(start.0 - center.0);
        let target_angle = (target.1 - center.1).atan2(target.0 - center.0);
        let angle = target_angle - start_angle;
        let angle = if snap_angle {
            (angle / (std::f64::consts::PI / 12.0)).round() * (std::f64::consts::PI / 12.0)
        } else {
            angle
        };
        Self::rotate_component_by_angle(project, original, angle)
    }
}
