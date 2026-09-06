
fn validate_component_master_topology(project: &FontProject, base_id: &str) -> Result<(), String> {
    for glyph in project.glyphs.values() {
        let Some(base) = glyph.layers.get(base_id) else {
            continue;
        };
        for master in &project.masters {
            let Some(layer) = glyph.layers.get(&master.id) else {
                continue;
            };
            if base.components.len() != layer.components.len()
                || base
                    .components
                    .iter()
                    .zip(&layer.components)
                    .any(|(a, b)| a.base != b.base)
            {
                return Err(format!(
                    "グリフ '{}' のマスター '{}' 間でコンポーネント構造が一致しません",
                    glyph.name, master.name
                ));
            }
        }
    }
    Ok(())
}
