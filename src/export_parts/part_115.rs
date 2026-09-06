
fn validate_component_master_transforms(
    project: &FontProject,
    base_id: &str,
) -> Result<(), String> {
    for glyph in project.glyphs.values() {
        let Some(base) = glyph.layers.get(base_id) else {
            continue;
        };
        for master in &project.masters {
            let Some(layer) = glyph.layers.get(&master.id) else {
                continue;
            };
            if base.components.iter().zip(&layer.components).any(|(a, b)| {
                a.base != b.base
                    || a.x_scale != b.x_scale
                    || a.xy_scale != b.xy_scale
                    || a.yx_scale != b.yx_scale
                    || a.y_scale != b.y_scale
            }) {
                return Err(format!(
                    "グリフ '{}' のマスター '{}' でコンポーネントの変形が一致しません",
                    glyph.name, master.name
                ));
            }
        }
    }
    Ok(())
}
