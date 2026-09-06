
fn append_svg_gradient_defs<'a>(
    svg: &mut String,
    gradients: impl Iterator<Item = (usize, &'a crate::font_data::ColorGradient)>,
    palette: &[[u8; 4]],
) {
    let gradients = gradients.collect::<Vec<_>>();
    if gradients.is_empty() {
        return;
    }
    svg.push_str("<defs>\n");
    for (index, gradient) in gradients {
        write_svg_gradient_def(
            svg,
            &format!("glyph-studio-gradient-{index}"),
            gradient,
            palette,
        );
    }
    svg.push_str("</defs>\n");
}
