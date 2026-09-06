
/// Exports one static CFF/OpenType font per master into a directory.
pub fn export_all_otf_for_masters(
    project: &FontProject,
    directory: &Path,
) -> Result<usize, String> {
    if project.masters.is_empty() {
        return Err("出力対象のマスターがありません".into());
    }
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("OTF出力先を作成できません: {error}"))?;
    let mut used = std::collections::HashSet::new();
    for (index, master) in project.masters.iter().enumerate() {
        let base: String = master
            .name
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || ".-_".contains(character) {
                    character
                } else {
                    '_'
                }
            })
            .collect();
        let base = if base.is_empty() {
            format!("master-{}", index + 1)
        } else {
            base
        };
        let mut filename = base.clone();
        let mut suffix = 2;
        while !used.insert(filename.clone()) {
            filename = format!("{base}_{suffix}");
            suffix += 1;
        }
        export_otf_for_master(
            project,
            &master.id,
            &directory.join(format!("{filename}.otf")),
        )?;
    }
    Ok(project.masters.len())
}
