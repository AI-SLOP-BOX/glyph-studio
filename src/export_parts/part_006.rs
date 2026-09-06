
fn write_svg_gradient_def(
    svg: &mut String,
    id: &str,
    gradient: &crate::font_data::ColorGradient,
    palette: &[[u8; 4]],
) {
    let (tag, attributes) = match gradient.kind {
        crate::font_data::ColorGradientKind::Linear => (
            "linearGradient",
            format!(
                "gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"",
                gradient.x0, gradient.y0, gradient.x1, gradient.y1
            ),
        ),
        crate::font_data::ColorGradientKind::Radial => (
            "radialGradient",
            format!(
                "gradientUnits=\"userSpaceOnUse\" cx=\"{}\" cy=\"{}\" r=\"{}\" fx=\"{}\" fy=\"{}\"",
                gradient.x1, gradient.y1, gradient.radius1, gradient.x0, gradient.y0
            ),
        ),
        crate::font_data::ColorGradientKind::Sweep => (
            "linearGradient",
            format!(
                "gradientUnits=\"userSpaceOnUse\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"",
                gradient.x0, gradient.y0, gradient.x1, gradient.y1
            ),
        ),
    };
    let spread = match gradient.extend {
        crate::font_data::ColorGradientExtend::Pad => "pad",
        crate::font_data::ColorGradientExtend::Repeat => "repeat",
        crate::font_data::ColorGradientExtend::Reflect => "reflect",
    };
    writeln!(
        svg,
        "<{tag} id=\"{id}\" spreadMethod=\"{spread}\" {attributes}>"
    )
    .ok();
    for stop in gradient.effective_stops() {
        let color = palette
            .get(usize::from(stop.palette_index))
            .copied()
            .unwrap_or([0, 0, 0, 0]);
        writeln!(
            svg,
            "<stop offset=\"{}\" stop-color=\"#{:02x}{:02x}{:02x}\" stop-opacity=\"{}\" />",
            stop.offset,
            color[0],
            color[1],
            color[2],
            (f64::from(color[3]) / 255.0 * stop.alpha.clamp(0.0, 1.0)).clamp(0.0, 1.0)
        )
        .ok();
    }
    writeln!(svg, "</{tag}>").ok();
}
