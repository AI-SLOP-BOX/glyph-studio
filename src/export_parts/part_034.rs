
/// Writes a static CFF2/OpenType font using the selected base master.
pub fn export_otf_cff2(project: &FontProject, path: &Path) -> Result<(), String> {
    let master_id = project
        .masters
        .iter()
        .find(|master| master.id == project.default_master_id)
        .or_else(|| project.masters.first())
        .map(|master| master.id.clone())
        .ok_or_else(|| "CFF2には基準マスターが必要です".to_string())?;
    let mut selected = project.clone();
    for glyph in selected.glyphs.values_mut() {
        if let Some(layer) = glyph.layers.get(&master_id).cloned() {
            glyph.width = layer.width;
            glyph.contours = layer.contours;
            glyph.components = layer.components;
            glyph.anchors = layer.anchors;
        }
    }
    selected.default_master_id = master_id.clone();
    let temp = std::env::temp_dir().join(format!(
        "glyph-studio-cff2-{}-{:?}.ttf",
        std::process::id(),
        std::thread::current().id()
    ));
    export_ttf_for_master(&selected, &master_id, &temp)?;
    let sfnt = std::fs::read(&temp).map_err(|error| error.to_string())?;
    let _ = std::fs::remove_file(&temp);
    let mut charstrings = vec![Vec::new()];
    for name in selected.glyph_names_sorted() {
        charstrings.push(cff::encode_project_glyph_cff2(&selected, name)?);
    }
    let cff2_table = cff::build_minimal_cff2(&charstrings)?;
    let otf = cff::rebuild_sfnt_with_table(&sfnt, *b"OTTO", *b"CFF2", &cff2_table)?;
    std::fs::write(path, otf).map_err(|error| error.to_string())
}
