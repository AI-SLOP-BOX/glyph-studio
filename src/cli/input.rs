use glyph_studio::io;
use std::path::Path;

pub(super) fn load_cli_project(
    path: &Path,
) -> Result<glyph_studio::font_data::FontProject, String> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
        .as_deref()
    {
        Some("json") => io::load_project(path),
        Some("glyphs") => io::load_glyphs(path),
        Some("ufo") => io::load_ufo(path),
        Some("ttf") | Some("otf") => io::load_ttf(path),
        Some("woff") => io::load_woff(path),
        Some("woff2") => io::load_woff2(path),
        _ => {
            Err("入力形式は json / glyphs / ufo / ttf / otf / woff / woff2 に対応しています".into())
        }
    }
}
