
fn apply_conditional_layers(
    project: &mut FontProject,
    axis_values: &std::collections::HashMap<String, f64>,
) {
    let names: Vec<String> = project.conditional_layers.keys().cloned().collect();
    for name in names {
        let Some(layer) = project
            .conditional_layer_for_glyph(&name, axis_values)
            .map(|layer| layer.layer.clone())
        else {
            continue;
        };
        if let Some(glyph) = project.glyphs.get_mut(&name) {
            glyph.width = layer.width;
            glyph.contours = layer.contours;
            glyph.components = layer.components;
            glyph.anchors = layer.anchors;
        }
    }
}
