
fn append_layer_contours(
    project: &FontProject,
    name: &str,
    master_id: Option<&str>,
    transform: Transform,
    stack: &mut Vec<String>,
    output: &mut Vec<Vec<fonttools::glyf::Point>>,
) -> Result<(), String> {
    if stack.iter().any(|item| item == name) {
        return Err(format!("コンポーネント循環参照: {}", stack.join(" -> ")));
    }
    let glyph = project
        .glyphs
        .get(name)
        .ok_or_else(|| format!("参照グリフ '{}' がありません", name))?;
    let (contours, components) = master_id
        .and_then(|id| glyph.layers.get(id))
        .map(|layer| (layer.contours.clone(), layer.components.clone()))
        .unwrap_or_else(|| (glyph.contours.clone(), glyph.components.clone()));
    stack.push(name.to_string());
    for contour in &contours {
        if contour.points.len() < 3 {
            return Err(format!("グリフ '{}' に不完全な輪郭があります", name));
        }
        output.push(
            contour
                .points
                .iter()
                .map(|point| {
                    Ok(fonttools::glyf::Point {
                        x: checked_i16(
                            transform.0 * point.x + transform.1 * point.y + transform.4,
                            "可変コンポーネントX座標",
                        )?,
                        y: checked_i16(
                            transform.2 * point.x + transform.3 * point.y + transform.5,
                            "可変コンポーネントY座標",
                        )?,
                        on_curve: point.point_type == PointType::OnCurve,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
        );
    }
    for component in &components {
        let child_transform = (
            transform.0 * component.x_scale + transform.1 * component.yx_scale,
            transform.0 * component.xy_scale + transform.1 * component.y_scale,
            transform.2 * component.x_scale + transform.3 * component.yx_scale,
            transform.2 * component.xy_scale + transform.3 * component.y_scale,
            transform.0 * component.x_offset + transform.1 * component.y_offset + transform.4,
            transform.2 * component.x_offset + transform.3 * component.y_offset + transform.5,
        );
        append_layer_contours(
            project,
            &component.base,
            master_id,
            child_transform,
            stack,
            output,
        )?;
    }
    stack.pop();
    Ok(())
}
