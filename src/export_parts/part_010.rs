
fn append_svg_nested_color_layers(
    project: &FontProject,
    base_name: &str,
    palette: &[[u8; 4]],
    path: &mut Vec<usize>,
    stack: &mut Vec<String>,
    svg: &mut String,
) -> Result<(), String> {
    if stack.iter().any(|item| item == base_name) {
        return Err(format!(
            "カラーグリフ循環参照: {} -> {}",
            stack.join(" -> "),
            base_name
        ));
    }
    let layers = project
        .color_layers
        .get(base_name)
        .ok_or_else(|| format!("カラーグリフ '{}' がありません", base_name))?;
    stack.push(base_name.to_string());
    for (index, layer) in layers.iter().enumerate() {
        path.push(index);
        let color = palette
            .get(usize::from(layer.palette_index))
            .copied()
            .unwrap_or([0, 0, 0, 255]);
        let is_nested = project.color_layers.contains_key(&layer.glyph);
        let opacity = if is_nested || layer.gradient.is_some() {
            layer.alpha.clamp(0.0, 1.0)
        } else {
            (f64::from(color[3]) / 255.0 * layer.alpha.clamp(0.0, 1.0)).clamp(0.0, 1.0)
        };
        let transform = svg_color_layer_transform(project, base_name, index);
        let fill = layer.gradient.as_ref().map_or_else(
            || {
                if is_nested {
                    "none".to_string()
                } else {
                    format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
                }
            },
            |_| format!("url(#{})", nested_svg_gradient_id(path)),
        );
        writeln!(
            svg,
            "<g fill=\"{fill}\" fill-opacity=\"{opacity:.6}\" fill-rule=\"nonzero\"{transform}>"
        )
        .map_err(|error| error.to_string())?;
        if project.color_layers.contains_key(&layer.glyph) {
            append_svg_nested_color_layers(project, &layer.glyph, palette, path, stack, svg)?;
        } else {
            append_svg_contours(
                project,
                &layer.glyph,
                (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
                stack,
                svg,
            )?;
        }
        svg.push_str("</g>\n");
        path.pop();
    }
    stack.pop();
    Ok(())
}
