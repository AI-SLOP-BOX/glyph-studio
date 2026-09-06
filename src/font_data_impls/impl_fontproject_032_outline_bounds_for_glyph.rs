use super::*;

impl FontProject {
    /// Returns the bounds of authored outlines and recursively referenced
    /// components in glyph coordinates.
    pub fn outline_bounds_for_glyph(&self, name: &str) -> Option<(f64, f64, f64, f64)> {
        fn collect(
            project: &FontProject,
            name: &str,
            transform: (f64, f64, f64, f64, f64, f64),
            stack: &mut std::collections::HashSet<String>,
            bounds: &mut Option<(f64, f64, f64, f64)>,
        ) {
            let Some(glyph) = project.glyphs.get(name) else {
                return;
            };
            if !stack.insert(name.to_string()) {
                return;
            }
            for point in glyph
                .contours
                .iter()
                .flat_map(|contour| contour.points.iter())
            {
                let x = transform.0 * point.x + transform.1 * point.y + transform.4;
                let y = transform.2 * point.x + transform.3 * point.y + transform.5;
                if !x.is_finite() || !y.is_finite() {
                    continue;
                }
                *bounds = Some(match *bounds {
                    Some((min_x, min_y, max_x, max_y)) => {
                        (min_x.min(x), min_y.min(y), max_x.max(x), max_y.max(y))
                    }
                    None => (x, y, x, y),
                });
            }
            for component in &glyph.components {
                let next = (
                    transform.0 * component.x_scale + transform.1 * component.yx_scale,
                    transform.0 * component.xy_scale + transform.1 * component.y_scale,
                    transform.2 * component.x_scale + transform.3 * component.yx_scale,
                    transform.2 * component.xy_scale + transform.3 * component.y_scale,
                    transform.0 * component.x_offset
                        + transform.1 * component.y_offset
                        + transform.4,
                    transform.2 * component.x_offset
                        + transform.3 * component.y_offset
                        + transform.5,
                );
                collect(project, &component.base, next, stack, bounds);
            }
            stack.remove(name);
        }
        let mut bounds = None;
        collect(
            self,
            name,
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut std::collections::HashSet::new(),
            &mut bounds,
        );
        bounds
    }
}
