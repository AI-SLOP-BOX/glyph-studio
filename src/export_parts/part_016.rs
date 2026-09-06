
fn append_svg_contours(
    project: &FontProject,
    name: &str,
    transform: Transform,
    stack: &mut Vec<String>,
    svg: &mut String,
) -> Result<(), String> {
    if stack.iter().any(|item| item == name) {
        return Err(format!("コンポーネント循環参照: {}", stack.join(" -> ")));
    }
    let glyph = project
        .glyphs
        .get(name)
        .ok_or_else(|| format!("参照グリフ '{}' がありません", name))?;
    stack.push(name.to_string());
    let map = |p: kurbo::Point| {
        (
            transform.0 * p.x + transform.1 * p.y + transform.4,
            transform.2 * p.x + transform.3 * p.y + transform.5,
        )
    };
    for contour in &glyph.contours {
        write!(svg, "<path d=\"").map_err(|e| e.to_string())?;
        for element in contour.to_bezpath().segments() {
            match element {
                kurbo::PathSeg::Line(line) => {
                    let a = map(line.p0);
                    let b = map(line.p1);
                    write!(svg, "M {} {} L {} {} ", a.0, -a.1, b.0, -b.1)
                }
                kurbo::PathSeg::Quad(q) => {
                    let a = map(q.p0);
                    let b = map(q.p1);
                    let c = map(q.p2);
                    write!(
                        svg,
                        "M {} {} Q {} {} {} {} ",
                        a.0, -a.1, b.0, -b.1, c.0, -c.1
                    )
                }
                kurbo::PathSeg::Cubic(c) => {
                    let a = map(c.p0);
                    let b = map(c.p1);
                    let d = map(c.p2);
                    let e = map(c.p3);
                    write!(
                        svg,
                        "M {} {} C {} {} {} {} {} {} ",
                        a.0, -a.1, b.0, -b.1, d.0, -d.1, e.0, -e.1
                    )
                }
            }
            .map_err(|e| e.to_string())?;
        }
        svg.push_str("Z\"/>\n");
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
        append_svg_contours(project, &component.base, t, stack, svg)?;
    }
    stack.pop();
    Ok(())
}
