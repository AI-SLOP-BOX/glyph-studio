
#[allow(dead_code)]
pub fn export_all_svg(project: &FontProject, directory: &Path) -> Result<usize, String> {
    export_all_svg_with_palette(project, 0, directory)
}
