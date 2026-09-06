
/// Writes a WOFF2 wrapper around the generated TrueType font.
///
/// Keeping the TrueType export path for multi-master projects preserves
/// variable-font tables such as `fvar`, `gvar`, `HVAR`, and `MVAR`. Static
/// callers retain the established CFF-based WOFF2 path.
pub fn export_woff2(project: &FontProject, path: &Path) -> Result<(), String> {
    let temp = std::env::temp_dir().join(format!(
        "glyph-studio-woff2-{}-{:?}.ttf",
        std::process::id(),
        std::thread::current().id()
    ));
    if project.masters.len() >= 2 {
        export_ttf(project, &temp)?;
    } else {
        export_otf(project, &temp)?;
    }
    let sfnt = match std::fs::read(&temp) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = std::fs::remove_file(&temp);
            return Err(error.to_string());
        }
    };
    let _ = std::fs::remove_file(&temp);
    let woff = oxifont_webfont::encode_woff2(&sfnt)
        .map_err(|error| format!("WOFF2圧縮に失敗しました: {error}"))?;
    std::fs::write(path, woff).map_err(|error| error.to_string())
}
