
#[allow(dead_code)]
pub fn export_svg(project: &FontProject, glyph_name: &str, path: &Path) -> Result<(), String> {
    export_svg_with_palette(project, glyph_name, 0, path)
}
