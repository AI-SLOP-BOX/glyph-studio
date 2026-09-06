
pub fn save_ufo(project: &FontProject, path: &Path) -> Result<(), String> {
    let font = project.to_norad()?;
    font.save(path).map_err(|e| format!("UFO保存エラー: {}", e))
}
