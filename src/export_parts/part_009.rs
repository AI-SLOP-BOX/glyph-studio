
fn append_svg_nested_gradient_defs(
    project: &FontProject,
    base_name: &str,
    palette: &[[u8; 4]],
    path: &mut Vec<usize>,
    stack: &mut Vec<String>,
    svg: &mut String,
) -> Result<bool, String> {
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
    let is_root = stack.is_empty();
    stack.push(base_name.to_string());
    let mut found = false;
    for (index, layer) in layers.iter().enumerate() {
        path.push(index);
        if !is_root {
            if let Some(gradient) = layer.gradient.as_ref() {
                write_svg_gradient_def(svg, &nested_svg_gradient_id(path), gradient, palette);
                found = true;
            }
        }
        if project.color_layers.contains_key(&layer.glyph) {
            found |=
                append_svg_nested_gradient_defs(project, &layer.glyph, palette, path, stack, svg)?;
        }
        path.pop();
    }
    stack.pop();
    Ok(found)
}
