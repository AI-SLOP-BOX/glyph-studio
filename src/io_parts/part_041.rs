
#[allow(clippy::too_many_arguments)]
fn import_colr_v1_paint(
    colr: &read_fonts::tables::colr::Colr<'_>,
    paint: read_fonts::tables::colr::Paint<'_>,
    names: &[String],
    leaf_glyph: Option<u16>,
    transform: Option<crate::font_data::ColorLayerTransform>,
    output: &mut Vec<crate::font_data::ColorLayer>,
    transforms: &mut Vec<Option<crate::font_data::ColorLayerTransform>>,
    depth: usize,
) {
    if depth > 32 {
        return;
    }
    use read_fonts::tables::colr::Paint;
    match paint {
        Paint::ColrLayers(layers) => {
            let first = layers.first_layer_index() as usize;
            for index in first..first.saturating_add(usize::from(layers.num_layers())) {
                if let Ok((child, _)) = colr.v1_layer(index) {
                    import_colr_v1_paint(
                        colr,
                        child,
                        names,
                        leaf_glyph,
                        transform,
                        output,
                        transforms,
                        depth + 1,
                    );
                }
            }
        }
        Paint::Glyph(glyph) => {
            if let Ok(child) = glyph.paint() {
                import_colr_v1_paint(
                    colr,
                    child,
                    names,
                    Some(glyph.glyph_id().to_u16()),
                    transform,
                    output,
                    transforms,
                    depth + 1,
                );
            }
        }
        Paint::ColrGlyph(glyph) => {
            if let Ok(Some((child, _))) = colr.v1_base_glyph(glyph.glyph_id().to_u32().into()) {
                import_colr_v1_paint(
                    colr,
                    child,
                    names,
                    leaf_glyph,
                    transform,
                    output,
                    transforms,
                    depth + 1,
                );
            }
        }
        Paint::Transform(item) => {
            if let Ok(matrix) = item.transform() {
                let transform = crate::font_data::ColorLayerTransform {
                    xx: matrix.xx().to_f32() as f64,
                    yx: matrix.yx().to_f32() as f64,
                    xy: matrix.xy().to_f32() as f64,
                    yy: matrix.yy().to_f32() as f64,
                    dx: matrix.dx().to_f32() as f64,
                    dy: matrix.dy().to_f32() as f64,
                };
                if let Ok(child) = item.paint() {
                    import_colr_v1_paint(
                        colr,
                        child,
                        names,
                        leaf_glyph,
                        Some(transform),
                        output,
                        transforms,
                        depth + 1,
                    );
                }
            }
        }
        Paint::Solid(item) => {
            if let Some(glyph_id) = leaf_glyph {
                if let Some(glyph) = names.get(usize::from(glyph_id)) {
                    output.push(crate::font_data::ColorLayer {
                        glyph: glyph.clone(),
                        palette_index: item.palette_index(),
                        gradient: None,
                        alpha: f64::from(item.alpha().to_f32()),
                    });
                    transforms.push(transform);
                }
            }
        }
        Paint::LinearGradient(item) => {
            if let Ok(line) = item.color_line() {
                let gradient = imported_color_line(
                    &line,
                    crate::font_data::ColorGradientKind::Linear,
                    f64::from(item.x0().to_i16()),
                    f64::from(item.y0().to_i16()),
                    f64::from(item.x1().to_i16()),
                    f64::from(item.y1().to_i16()),
                    f64::from(item.x2().to_i16()),
                    f64::from(item.y2().to_i16()),
                    0.0,
                    0.0,
                    0.0,
                    360.0,
                );
                push_imported_color_layer(
                    names,
                    leaf_glyph,
                    transform,
                    gradient.start_palette_index,
                    Some(gradient),
                    output,
                    transforms,
                );
            }
        }
        Paint::RadialGradient(item) => {
            if let Ok(line) = item.color_line() {
                let gradient = imported_color_line(
                    &line,
                    crate::font_data::ColorGradientKind::Radial,
                    f64::from(item.x0().to_i16()),
                    f64::from(item.y0().to_i16()),
                    f64::from(item.x1().to_i16()),
                    f64::from(item.y1().to_i16()),
                    0.0,
                    0.0,
                    f64::from(item.radius0().to_u16()),
                    f64::from(item.radius1().to_u16()),
                    0.0,
                    360.0,
                );
                push_imported_color_layer(
                    names,
                    leaf_glyph,
                    transform,
                    gradient.start_palette_index,
                    Some(gradient),
                    output,
                    transforms,
                );
            }
        }
        Paint::SweepGradient(item) => {
            if let Ok(line) = item.color_line() {
                let gradient = imported_color_line(
                    &line,
                    crate::font_data::ColorGradientKind::Sweep,
                    f64::from(item.center_x().to_i16()),
                    f64::from(item.center_y().to_i16()),
                    f64::from(item.center_x().to_i16()),
                    f64::from(item.center_y().to_i16()),
                    0.0,
                    0.0,
                    0.0,
                    0.0,
                    f64::from(item.start_angle().to_f32()) * 180.0,
                    f64::from(item.end_angle().to_f32()) * 180.0,
                );
                push_imported_color_layer(
                    names,
                    leaf_glyph,
                    transform,
                    gradient.start_palette_index,
                    Some(gradient),
                    output,
                    transforms,
                );
            }
        }
        Paint::Composite(item) => {
            // The editable model currently represents a color glyph as an
            // ordered layer list rather than a general paint graph. Flatten
            // the backdrop first and the source second, preserving the
            // compositing order so common COLR v1 composites remain editable
            // instead of being discarded as opaque raw data. The exact blend
            // mode is intentionally not guessed here; unsupported blend modes
            // are represented by the same ordered layers and can be rebuilt
            // safely as SrcOver on export.
            if let Ok(backdrop) = item.backdrop_paint() {
                import_colr_v1_paint(
                    colr,
                    backdrop,
                    names,
                    leaf_glyph,
                    transform,
                    output,
                    transforms,
                    depth + 1,
                );
            }
            if let Ok(source) = item.source_paint() {
                import_colr_v1_paint(
                    colr,
                    source,
                    names,
                    leaf_glyph,
                    transform,
                    output,
                    transforms,
                    depth + 1,
                );
            }
        }
        _ => {}
    }
}
