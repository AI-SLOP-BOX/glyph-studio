
pub fn export_woff_for_master(
    project: &FontProject,
    master_id: &str,
    path: &Path,
) -> Result<(), String> {
    let master = project
        .masters
        .iter()
        .find(|master| master.id == master_id)
        .cloned()
        .ok_or_else(|| format!("マスター '{}' がありません", master_id))?;
    let mut single = project.clone();
    let mut axis_values = master.axes.clone();
    axis_values.insert("wght".into(), master.weight);
    axis_values.insert("wdth".into(), master.width);
    apply_conditional_layers(&mut single, &axis_values);
    for glyph in single.glyphs.values_mut() {
        if let Some(layer) = glyph.layers.get(master_id).cloned() {
            glyph.width = layer.width;
            glyph.contours = layer.contours;
            glyph.components = layer.components;
            glyph.anchors = layer.anchors;
        }
        glyph.layers.clear();
    }
    single.masters = vec![master.clone()];
    single.default_master_id = master.id;
    export_woff(&single, path)
}
