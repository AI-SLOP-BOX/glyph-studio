
pub fn export_otf_for_master(
    project: &FontProject,
    master_id: &str,
    path: &Path,
) -> Result<(), String> {
    if !project.masters.iter().any(|master| master.id == master_id) {
        return Err(format!("マスター '{}' がありません", master_id));
    }
    let mut selected = project.clone();
    selected.default_master_id = master_id.to_string();
    export_otf(&selected, path)
}
