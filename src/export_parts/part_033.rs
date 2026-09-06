
pub fn export_otf(project: &FontProject, path: &Path) -> Result<(), String> {
    let master_id = project
        .masters
        .iter()
        .find(|master| master.id == project.default_master_id)
        .or_else(|| project.masters.first())
        .map(|master| master.id.clone())
        .ok_or_else(|| "OTFには基準マスターが必要です".to_string())?;
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
        "glyph-studio-otf-{}-{:?}.ttf",
        std::process::id(),
        std::thread::current().id()
    ));
    export_ttf_for_master(&selected, &master_id, &temp)?;
    let sfnt = std::fs::read(&temp).map_err(|error| error.to_string())?;
    let _ = std::fs::remove_file(&temp);
    let mut charstrings = vec![cff::encode_type2_with_width(
        selected.metadata.units_per_em,
        &[],
    )?]; // .notdef
    for name in selected.glyph_names_sorted() {
        charstrings.push(cff::encode_project_glyph(&selected, name)?);
    }
    let cff_table = cff::build_minimal_cff(&selected.metadata.family_name, &charstrings)?;
    let otf = cff::rebuild_sfnt_with_table(&sfnt, *b"OTTO", *b"CFF ", &cff_table)?;
    let vorg = build_vorg(&selected, &master_id)?;
    let otf = cff::rebuild_sfnt_with_table(&otf, *b"OTTO", *b"VORG", &vorg)?;
    std::fs::write(path, otf).map_err(|error| error.to_string())
}
