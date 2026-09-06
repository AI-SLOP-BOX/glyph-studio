fn parse_glyphs_node(value: &str) -> Option<crate::font_data::ContourPoint> {
    let mut parts = value.split_whitespace();
    let x = parts.next()?.parse().ok()?;
    let y = parts.next()?.parse().ok()?;
    let kind = parts.next().unwrap_or("LINE");
    let smooth = parts.any(|part| part == "SMOOTH");
    let mut point = if kind == "OFFCURVE" {
        crate::font_data::ContourPoint::off_curve(x, y)
    } else {
        crate::font_data::ContourPoint::on_curve(x, y)
    };
    point.smooth = smooth;
    Some(point)
}
