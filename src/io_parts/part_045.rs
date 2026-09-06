
/// Imports all SVG path elements as glyph contours.
pub fn load_svg(path: &Path) -> Result<FontProject, String> {
    let source = std::fs::read_to_string(path).map_err(|e| format!("SVG読み込みエラー: {e}"))?;
    let mut path_data = Vec::new();
    for (offset, _) in source.match_indices("d=") {
        if offset > 0
            && source[..offset]
                .chars()
                .next_back()
                .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            continue;
        }
        let rest = &source[offset + 2..];
        let Some(quote) = rest.chars().next() else {
            continue;
        };
        if quote != '"' && quote != '\'' {
            continue;
        }
        if let Some((value, _)) = rest[quote.len_utf8()..].split_once(quote) {
            path_data.push(value);
        }
    }
    if path_data.is_empty() {
        return Err("SVGにパスがありません".into());
    }
    let d = path_data.join(" ");
    let mut contours = Vec::new();
    let mut points = Vec::new();
    let mut current = (0.0, 0.0);
    let mut has_on_curve = 0;
    let mut last_cubic_control = None;
    let mut last_quad_control = None;
    let flush = |points: &mut Vec<crate::font_data::ContourPoint>,
                 contours: &mut Vec<crate::font_data::Contour>,
                 has_on_curve: &mut usize| {
        if *has_on_curve >= 3 {
            contours.push(crate::font_data::Contour {
                points: std::mem::take(points),
            });
        } else {
            points.clear();
        }
        *has_on_curve = 0;
    };
    for segment in svgtypes::PathParser::from(d.as_str()) {
        let segment = segment.map_err(|error| format!("SVGパス解析エラー: {error}"))?;
        match segment {
            svgtypes::PathSegment::MoveTo { abs, x, y } => {
                flush(&mut points, &mut contours, &mut has_on_curve);
                current = if abs {
                    (x, y)
                } else {
                    (current.0 + x, current.1 + y)
                };
                points.push(crate::font_data::ContourPoint::on_curve(
                    current.0, current.1,
                ));
                has_on_curve += 1;
                last_cubic_control = None;
                last_quad_control = None;
            }
            svgtypes::PathSegment::LineTo { abs, x, y } => {
                current = if abs {
                    (x, y)
                } else {
                    (current.0 + x, current.1 + y)
                };
                points.push(crate::font_data::ContourPoint::on_curve(
                    current.0, current.1,
                ));
                has_on_curve += 1;
                last_cubic_control = None;
                last_quad_control = None;
            }
            svgtypes::PathSegment::HorizontalLineTo { abs, x } => {
                current.0 = if abs { x } else { current.0 + x };
                points.push(crate::font_data::ContourPoint::on_curve(
                    current.0, current.1,
                ));
                has_on_curve += 1;
                last_cubic_control = None;
                last_quad_control = None;
            }
            svgtypes::PathSegment::VerticalLineTo { abs, y } => {
                current.1 = if abs { y } else { current.1 + y };
                points.push(crate::font_data::ContourPoint::on_curve(
                    current.0, current.1,
                ));
                has_on_curve += 1;
                last_cubic_control = None;
                last_quad_control = None;
            }
            svgtypes::PathSegment::Quadratic { abs, x1, y1, x, y } => {
                let control = if abs {
                    (x1, y1)
                } else {
                    (current.0 + x1, current.1 + y1)
                };
                current = if abs {
                    (x, y)
                } else {
                    (current.0 + x, current.1 + y)
                };
                points.push(crate::font_data::ContourPoint::off_curve(
                    control.0, control.1,
                ));
                points.push(crate::font_data::ContourPoint::on_curve(
                    current.0, current.1,
                ));
                has_on_curve += 1;
                last_quad_control = Some(control);
                last_cubic_control = None;
            }
            svgtypes::PathSegment::CurveTo {
                abs,
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let first = if abs {
                    (x1, y1)
                } else {
                    (current.0 + x1, current.1 + y1)
                };
                let second = if abs {
                    (x2, y2)
                } else {
                    (current.0 + x2, current.1 + y2)
                };
                current = if abs {
                    (x, y)
                } else {
                    (current.0 + x, current.1 + y)
                };
                points.push(crate::font_data::ContourPoint::off_curve(first.0, first.1));
                points.push(crate::font_data::ContourPoint::off_curve(
                    second.0, second.1,
                ));
                points.push(crate::font_data::ContourPoint::on_curve(
                    current.0, current.1,
                ));
                has_on_curve += 1;
                last_cubic_control = Some(second);
                last_quad_control = None;
            }
            svgtypes::PathSegment::SmoothQuadratic { abs, x, y } => {
                let control = last_quad_control
                    .map(|(cx, cy)| (2.0 * current.0 - cx, 2.0 * current.1 - cy))
                    .unwrap_or(current);
                current = if abs {
                    (x, y)
                } else {
                    (current.0 + x, current.1 + y)
                };
                points.push(crate::font_data::ContourPoint::off_curve(
                    control.0, control.1,
                ));
                points.push(crate::font_data::ContourPoint::on_curve(
                    current.0, current.1,
                ));
                has_on_curve += 1;
                last_quad_control = Some(control);
                last_cubic_control = None;
            }
            svgtypes::PathSegment::SmoothCurveTo { abs, x2, y2, x, y } => {
                let first = last_cubic_control
                    .map(|(cx, cy)| (2.0 * current.0 - cx, 2.0 * current.1 - cy))
                    .unwrap_or(current);
                let second = if abs {
                    (x2, y2)
                } else {
                    (current.0 + x2, current.1 + y2)
                };
                current = if abs {
                    (x, y)
                } else {
                    (current.0 + x, current.1 + y)
                };
                points.push(crate::font_data::ContourPoint::off_curve(first.0, first.1));
                points.push(crate::font_data::ContourPoint::off_curve(
                    second.0, second.1,
                ));
                points.push(crate::font_data::ContourPoint::on_curve(
                    current.0, current.1,
                ));
                has_on_curve += 1;
                last_cubic_control = Some(second);
                last_quad_control = None;
            }
            svgtypes::PathSegment::EllipticalArc {
                abs,
                rx,
                ry,
                x_axis_rotation,
                large_arc,
                sweep,
                x,
                y,
            } => {
                let endpoint = if abs {
                    (x, y)
                } else {
                    (current.0 + x, current.1 + y)
                };
                let svg_arc = SvgArc {
                    from: Point::new(current.0, current.1),
                    to: Point::new(endpoint.0, endpoint.1),
                    radii: Vec2::new(rx, ry),
                    x_rotation: x_axis_rotation.to_radians(),
                    large_arc,
                    sweep,
                };
                if let Some(arc) = Arc::from_svg_arc(&svg_arc) {
                    arc.to_cubic_beziers(0.1, |first, second, end| {
                        points.push(crate::font_data::ContourPoint::off_curve(first.x, first.y));
                        points.push(crate::font_data::ContourPoint::off_curve(
                            second.x, second.y,
                        ));
                        points.push(crate::font_data::ContourPoint::on_curve(end.x, end.y));
                        has_on_curve += 1;
                    });
                } else {
                    points.push(crate::font_data::ContourPoint::on_curve(
                        endpoint.0, endpoint.1,
                    ));
                    has_on_curve += 1;
                }
                current = endpoint;
                last_cubic_control = None;
                last_quad_control = None;
            }
            svgtypes::PathSegment::ClosePath { .. } => {
                flush(&mut points, &mut contours, &mut has_on_curve);
                last_cubic_control = None;
                last_quad_control = None;
            }
        }
    }
    flush(&mut points, &mut contours, &mut has_on_curve);
    if contours.is_empty() {
        return Err("有効なSVG輪郭がありません".into());
    }
    let name = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("imported")
        .to_string();
    let mut project = FontProject::new();
    let mut glyph = crate::font_data::GlyphData::new(name.clone(), None);
    glyph.contours = contours;
    glyph.width = project.metadata.units_per_em;
    project.glyphs.insert(name.clone(), glyph);
    project.glyph_order = vec![name];
    Ok(project)
}
