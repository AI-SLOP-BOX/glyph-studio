use super::*;

impl FontProject {
    pub fn anchors_for_glyph(&self, name: &str) -> Vec<GlyphAnchor> {
        fn collect(
            project: &FontProject,
            name: &str,
            transform: (f64, f64, f64, f64, f64, f64),
            stack: &mut std::collections::HashSet<String>,
            output: &mut Vec<GlyphAnchor>,
        ) {
            let Some(glyph) = project.glyphs.get(name) else {
                return;
            };
            if !stack.insert(name.to_string()) {
                return;
            }
            let map = |x: f64, y: f64| {
                (
                    transform.0 * x + transform.1 * y + transform.4,
                    transform.2 * x + transform.3 * y + transform.5,
                )
            };
            for anchor in &glyph.anchors {
                let (x, y) = map(anchor.x, anchor.y);
                output.push(GlyphAnchor {
                    name: anchor.name.clone(),
                    x,
                    y,
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
                collect(project, &component.base, next, stack, output);
            }
            stack.remove(name);
        }
        let mut output = Vec::new();
        collect(
            self,
            name,
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut std::collections::HashSet::new(),
            &mut output,
        );
        let base_names: std::collections::HashSet<String> = output
            .iter()
            .filter(|anchor| !anchor.name.starts_with('_'))
            .map(|anchor| anchor.name.clone())
            .collect();
        output.retain(|anchor| {
            !anchor.name.starts_with('_')
                || !base_names.contains(anchor.name.trim_start_matches('_'))
        });
        let mut seen = std::collections::HashSet::new();
        output.retain(|anchor| seen.insert(anchor.name.clone()));
        output
    }
}
