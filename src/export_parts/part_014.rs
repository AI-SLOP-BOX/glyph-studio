
#[allow(dead_code)]
pub fn export_all_svg_for_master(
    project: &FontProject,
    master_id: &str,
    directory: &Path,
) -> Result<usize, String> {
    if !project.masters.iter().any(|master| master.id == master_id) {
        return Err(format!("マスター '{}' がありません", master_id));
    }
    let mut selected = project.clone();
    for glyph in selected.glyphs.values_mut() {
        if let Some(layer) = glyph.layers.get(master_id).cloned() {
            glyph.width = layer.width;
            glyph.contours = layer.contours;
            glyph.components = layer.components;
            glyph.anchors = layer.anchors;
        }
    }
    export_all_svg(&selected, directory)
}
