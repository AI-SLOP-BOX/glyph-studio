use super::super::*;

impl GlyphStudioApp {
    pub(crate) fn component_visual_center(
        project: &FontProject,
        component: &GlyphComponent,
    ) -> Option<(f64, f64)> {
        let (min_x, min_y, max_x, max_y) = project.outline_bounds_for_glyph(&component.base)?;
        let corners = [
            (min_x, min_y),
            (min_x, max_y),
            (max_x, min_y),
            (max_x, max_y),
        ];
        let transformed = corners.into_iter().map(|(x, y)| {
            (
                component.x_scale * x + component.yx_scale * y + component.x_offset,
                component.xy_scale * x + component.y_scale * y + component.y_offset,
            )
        });
        let (mut min_x, mut min_y) = (f64::INFINITY, f64::INFINITY);
        let (mut max_x, mut max_y) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for (x, y) in transformed {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
        Some(((min_x + max_x) * 0.5, (min_y + max_y) * 0.5))
    }
}
