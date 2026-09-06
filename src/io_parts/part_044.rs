
/// Loads a WOFF2 file by decoding it to an SFNT font first.
pub fn load_woff2(path: &Path) -> Result<FontProject, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("WOFF2読み込みエラー: {e}"))?;
    let sfnt = oxifont_webfont::decode_woff2(&bytes)
        .map_err(|error| format!("WOFF2の展開に失敗しました: {error}"))?;
    let temp = std::env::temp_dir().join(format!(
        "glyph-studio-woff2-import-{}-{:?}.ttf",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::write(&temp, sfnt).map_err(|error| error.to_string())?;
    let result = load_ttf(&temp);
    let _ = std::fs::remove_file(temp);
    result
}
