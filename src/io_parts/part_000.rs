
fn parse_color_gradient(info: &plist::Dictionary) -> Option<ColorGradient> {
    let value = info.get("gradient")?.as_dictionary()?;
    let integer = |key: &str| {
        value
            .get(key)
            .and_then(|item| {
                item.as_signed_integer()
                    .or_else(|| item.as_unsigned_integer().map(|v| v as i64))
            })
            .and_then(|item| u16::try_from(item).ok())
    };
    let real = |key: &str| {
        value.get(key).and_then(|item| {
            item.as_real()
                .or_else(|| item.as_signed_integer().map(|v| v as f64))
        })
    };
    let stops = value
        .get("stops")
        .and_then(plist::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(plist::Value::as_dictionary)
        .filter_map(|stop| {
            Some(crate::font_data::ColorGradientStop {
                offset: real_from(stop, "offset")?,
                palette_index: integer_from(stop, "paletteIndex")?,
                alpha: real_from(stop, "alpha").unwrap_or(1.0),
            })
        })
        .collect();
    Some(ColorGradient {
        start_palette_index: integer("startPaletteIndex")?,
        end_palette_index: integer("endPaletteIndex")?,
        kind: match value.get("kind").and_then(plist::Value::as_string) {
            Some("radial") => crate::font_data::ColorGradientKind::Radial,
            Some("sweep") => crate::font_data::ColorGradientKind::Sweep,
            _ => crate::font_data::ColorGradientKind::Linear,
        },
        extend: match value.get("extend").and_then(plist::Value::as_string) {
            Some("repeat") => crate::font_data::ColorGradientExtend::Repeat,
            Some("reflect") => crate::font_data::ColorGradientExtend::Reflect,
            _ => crate::font_data::ColorGradientExtend::Pad,
        },
        x0: real("x0")?,
        y0: real("y0")?,
        x1: real("x1")?,
        y1: real("y1")?,
        x2: real("x2").unwrap_or_else(|| real("x1").unwrap_or(0.0)),
        y2: real("y2").unwrap_or_else(|| real("y1").unwrap_or(0.0)),
        stops,
        radius0: real("radius0").unwrap_or(0.0),
        radius1: real("radius1").unwrap_or(0.0),
        start_angle: real("startAngle").unwrap_or(0.0),
        end_angle: real("endAngle").unwrap_or(360.0),
    })
}
