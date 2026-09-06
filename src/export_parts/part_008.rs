
fn nested_svg_gradient_id(path: &[usize]) -> String {
    let suffix = path
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join("-");
    format!("glyph-studio-nested-gradient-{suffix}")
}
