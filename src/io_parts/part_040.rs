
#[allow(clippy::too_many_arguments)]
fn imported_color_line(
    line: &read_fonts::tables::colr::ColorLine<'_>,
    kind: crate::font_data::ColorGradientKind,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    radius0: f64,
    radius1: f64,
    start_angle: f64,
    end_angle: f64,
) -> crate::font_data::ColorGradient {
    let stops = line
        .color_stops()
        .iter()
        .map(|stop| crate::font_data::ColorGradientStop {
            offset: f64::from(stop.stop_offset().to_f32()),
            palette_index: stop.palette_index(),
            alpha: f64::from(stop.alpha().to_f32()),
        })
        .collect::<Vec<_>>();
    let start_palette_index = stops.first().map_or(0, |stop| stop.palette_index);
    let end_palette_index = stops
        .last()
        .map_or(start_palette_index, |stop| stop.palette_index);
    let extend = match line.extend() {
        read_fonts::tables::colr::Extend::Repeat => crate::font_data::ColorGradientExtend::Repeat,
        read_fonts::tables::colr::Extend::Reflect => crate::font_data::ColorGradientExtend::Reflect,
        _ => crate::font_data::ColorGradientExtend::Pad,
    };
    crate::font_data::ColorGradient {
        start_palette_index,
        end_palette_index,
        kind,
        extend,
        x0,
        y0,
        x1,
        y1,
        x2,
        y2,
        stops,
        radius0,
        radius1,
        start_angle,
        end_angle,
    }
}
