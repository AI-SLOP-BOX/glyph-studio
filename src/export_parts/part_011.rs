
pub fn export_svg_with_palette(
    project: &FontProject,
    glyph_name: &str,
    palette_index: usize,
    path: &Path,
) -> Result<(), String> {
    if !project.glyphs.contains_key(glyph_name) {
        return Err(format!("グリフ '{}' がありません", glyph_name));
    }
    let glyph_width = project.glyphs[glyph_name].width.max(1.0);
    let top = project.metadata.ascender.max(0.0);
    let bottom = project.metadata.descender.min(0.0);
    let mut svg = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 {} {} {}\">\n",
        -top,
        glyph_width,
        (top - bottom).max(1.0)
    );
    if let Some(layers) = project.color_layers.get(glyph_name) {
        let palette = project
            .color_palettes
            .get(palette_index)
            .or_else(|| project.color_palettes.first())
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        append_svg_gradient_defs(
            &mut svg,
            layers.iter().enumerate().filter_map(|(index, layer)| {
                layer.gradient.as_ref().map(|gradient| (index, gradient))
            }),
            palette,
        );
        let mut nested_definitions = String::new();
        if append_svg_nested_gradient_defs(
            project,
            glyph_name,
            palette,
            &mut Vec::new(),
            &mut Vec::new(),
            &mut nested_definitions,
        )? {
            svg.push_str("<defs>\n");
            svg.push_str(&nested_definitions);
            svg.push_str("</defs>\n");
        }
        for (index, layer) in layers.iter().enumerate() {
            let Some(color) = project
                .color_palettes
                .get(palette_index)
                .and_then(|palette| palette.get(usize::from(layer.palette_index)))
            else {
                continue;
            };
            let is_nested = project.color_layers.contains_key(&layer.glyph);
            let fill = layer.gradient.as_ref().map_or_else(
                || {
                    if is_nested {
                        "none".to_string()
                    } else {
                        format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
                    }
                },
                |_| format!("url(#glyph-studio-gradient-{index})"),
            );
            let opacity = if is_nested || layer.gradient.is_some() {
                layer.alpha.clamp(0.0, 1.0)
            } else {
                (f64::from(color[3]) / 255.0 * layer.alpha.clamp(0.0, 1.0)).clamp(0.0, 1.0)
            };
            let transform = svg_color_layer_transform(project, glyph_name, index);
            writeln!(
                svg,
                "<g fill=\"{fill}\" fill-opacity=\"{opacity:.6}\" fill-rule=\"nonzero\"{transform}>"
            )
            .map_err(|e| e.to_string())?;
            if project.color_layers.contains_key(&layer.glyph) {
                append_svg_nested_color_layers(
                    project,
                    &layer.glyph,
                    palette,
                    &mut vec![index],
                    &mut vec![glyph_name.to_string()],
                    &mut svg,
                )?;
            } else {
                append_svg_contours(
                    project,
                    &layer.glyph,
                    (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
                    &mut Vec::new(),
                    &mut svg,
                )?;
            }
            svg.push_str("</g>\n");
        }
    } else {
        svg.push_str("<g fill=\"black\" fill-rule=\"nonzero\">\n");
        append_svg_contours(
            project,
            glyph_name,
            (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
            &mut Vec::new(),
            &mut svg,
        )?;
        svg.push_str("</g>\n");
    }
    svg.push_str("</svg>\n");
    std::fs::write(path, svg).map_err(|e| format!("SVG保存エラー: {e}"))
}
