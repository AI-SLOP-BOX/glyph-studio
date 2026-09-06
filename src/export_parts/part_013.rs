
pub fn export_all_svg_with_palette(
    project: &FontProject,
    palette_index: usize,
    directory: &Path,
) -> Result<usize, String> {
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("SVG出力先を作成できません: {error}"))?;
    let mut exported = 0;
    let mut used_names = std::collections::HashSet::new();
    for glyph_name in project.glyph_names_sorted() {
        let base_name: String = glyph_name
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || ".-_".contains(character) {
                    character
                } else {
                    '_'
                }
            })
            .collect();
        if base_name.is_empty() {
            continue;
        }
        let mut safe_name = base_name.clone();
        let mut suffix = 2;
        while !used_names.insert(safe_name.clone()) {
            safe_name = format!("{base_name}_{suffix}");
            suffix += 1;
        }
        export_svg_with_palette(
            project,
            glyph_name,
            palette_index,
            &directory.join(format!("{safe_name}.svg")),
        )?;
        exported += 1;
    }
    Ok(exported)
}
