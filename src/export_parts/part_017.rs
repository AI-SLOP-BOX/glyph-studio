
fn append_contours(
    project: &FontProject,
    name: &str,
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
    stack.push(name.to_string());
    for contour in &glyph.contours {
        if contour.points.len() < 3 {
            return Err(format!("グリフ '{}' に不完全な輪郭があります", name));
        }
        output.push(
            contour
                .points
                .iter()
                .map(|point| {
                    let x = transform.0 * point.x + transform.1 * point.y + transform.4;
                    let y = transform.2 * point.x + transform.3 * point.y + transform.5;
                    Ok(fonttools::glyf::Point {
                        x: checked_i16(x, "コンポーネントX座標")?,
                        y: checked_i16(y, "コンポーネントY座標")?,
                        on_curve: point.point_type == PointType::OnCurve,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
        );
    }
    for component in &glyph.components {
        let t = (
            transform.0 * component.x_scale + transform.1 * component.yx_scale,
            transform.0 * component.xy_scale + transform.1 * component.y_scale,
            transform.2 * component.x_scale + transform.3 * component.yx_scale,
            transform.2 * component.xy_scale + transform.3 * component.y_scale,
            transform.0 * component.x_offset + transform.1 * component.y_offset + transform.4,
            transform.2 * component.x_offset + transform.3 * component.y_offset + transform.5,
        );
        append_contours(project, &component.base, t, stack, output)?;
    }
    stack.pop();
    Ok(())
}
