
pub fn save_project(project: &FontProject, path: &Path) -> Result<(), String> {
    let mut normalized = project.clone();
    normalized.normalize_glyph_order();
    normalized.normalize_masters();
    let json = serde_json::to_vec_pretty(&normalized)
        .map_err(|e| format!("プロジェクト変換エラー: {e}"))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "プロジェクト保存先のファイル名が不正です".to_string())?;
    let temporary = path.with_file_name(format!(".{file_name}.tmp"));
    std::fs::write(&temporary, json).map_err(|e| format!("プロジェクト保存エラー: {e}"))?;
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("プロジェクト保存エラー: {error}"));
    }
    Ok(())
}
