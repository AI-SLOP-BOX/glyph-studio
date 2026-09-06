
pub fn load_project(path: &Path) -> Result<FontProject, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("プロジェクト読み込みエラー: {e}"))?;
    let mut project: FontProject =
        serde_json::from_slice(&bytes).map_err(|e| format!("プロジェクト形式エラー: {e}"))?;
    project.normalize_glyph_order();
    project.normalize_masters();
    Ok(project)
}
