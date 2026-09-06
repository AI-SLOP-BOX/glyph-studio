
/// 出力先の拡張子に応じて、対応するフォント形式で書き出す。
pub fn export_by_extension(project: &FontProject, path: &Path) -> Result<(), String> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("ttf") => export_ttf(project, path),
        Some("otf") => export_otf(project, path),
        Some("woff") => export_woff(project, path),
        Some("woff2") => export_woff2(project, path),
        _ => Err("出力形式は ttf / otf / woff / woff2 に対応しています".into()),
    }
}
