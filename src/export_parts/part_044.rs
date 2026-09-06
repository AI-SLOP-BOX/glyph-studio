
pub fn export_all_woff_for_masters(
    project: &FontProject,
    directory: &Path,
) -> Result<usize, String> {
    if project.masters.is_empty() {
        return Err("マスターがありません".to_string());
    }
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let mut used = std::collections::HashSet::new();
    for (index, master) in project.masters.iter().enumerate() {
        let stem: String = master
            .name
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character
                } else {
                    '-'
                }
            })
            .collect();
        let stem = if stem.trim_matches('-').is_empty() {
            format!("master-{}", index + 1)
        } else {
            stem.trim_matches('-').to_string()
        };
        let mut output = directory.join(format!("{stem}.woff"));
        let mut suffix = 2;
        while output.exists() || !used.insert(output.clone()) {
            output = directory.join(format!("{stem}-{suffix}.woff"));
            suffix += 1;
        }
        export_woff_for_master(project, &master.id, &output)?;
    }
    Ok(project.masters.len())
}
