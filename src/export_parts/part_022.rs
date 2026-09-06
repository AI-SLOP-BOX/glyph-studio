
fn build_color_tables(
    project: &FontProject,
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Option<(Vec<u8>, Vec<u8>)> {
    let mut bases = Vec::new();
    let mut layers = Vec::new();
    let mut paint_layers = Vec::new();
    let mut color_glyphs: Vec<(&str, u16)> = project
        .color_layers
        .keys()
        .filter_map(|name| {
            glyph_ids
                .get(name.as_str())
                .copied()
                .map(|id| (name.as_str(), id))
        })
        .collect();
    color_glyphs.sort_unstable_by_key(|(_, base_id)| *base_id);
    for (name, base_id) in color_glyphs {
        let entries = project.color_layers.get(name)?;
        let first = u16::try_from(layers.len()).ok()?;
        for entry in entries {
            let &layer_id = glyph_ids.get(entry.glyph.as_str())?;
            layers.push((layer_id, entry.palette_index));
            paint_layers.push((
                layer_id,
                entry.palette_index,
                entry.gradient.clone(),
                entry.alpha,
                entry.gradient.is_none() && project.color_layers.contains_key(&entry.glyph),
                project
                    .color_layer_transforms
                    .get(name)
                    .and_then(|transforms| transforms.get(paint_layers.len()))
                    .copied()
                    .flatten(),
            ));
        }
        let count = u16::try_from(entries.len()).ok()?;
        if count > 0 {
            bases.push((base_id, first, count));
        }
    }
    if bases.is_empty() || project.color_palettes.is_empty() {
        return None;
    }
    if layers.len() > u16::MAX as usize {
        return None;
    }
    if layers.len() > u32::MAX as usize || bases.len() > u32::MAX as usize {
        return None;
    }
    let v0_base_offset = 34usize;
    let v0_layer_offset = v0_base_offset.checked_add(bases.len() * 6)?;
    let mut colr = Vec::with_capacity(v0_layer_offset + layers.len() * 4 + 256);
    put_u16(&mut colr, 1);
    put_u16(&mut colr, u16::try_from(bases.len()).ok()?);
    put_u32(&mut colr, u32::try_from(v0_base_offset).ok()?);
    put_u32(&mut colr, u32::try_from(v0_layer_offset).ok()?);
    put_u16(&mut colr, u16::try_from(layers.len()).ok()?);
    let base_glyph_list_offset_position = colr.len();
    colr.resize(34, 0);
    for (glyph, first, count) in &bases {
        put_u16(&mut colr, *glyph);
        put_u16(&mut colr, *first);
        put_u16(&mut colr, *count);
    }
    for (glyph, palette) in &layers {
        put_u16(&mut colr, *glyph);
        put_u16(&mut colr, *palette);
    }

    // COLR v1 keeps the v0 records above for older consumers and adds a
    // PaintColrLayers graph using PaintGlyph + PaintSolid for the same data.
    while colr.len() % 4 != 0 {
        colr.push(0);
    }
    let base_glyph_list_offset = colr.len();
    put_u32(&mut colr, u32::try_from(bases.len()).ok()?);
    for (glyph, _, _) in &bases {
        put_u16(&mut colr, *glyph);
        put_u32(&mut colr, 0);
    }
    let paint_colr_layers_offset = colr.len();
    for (_, first, count) in &bases {
        if *count > u8::MAX as u16 {
            return None;
        }
        put_u8(&mut colr, 1);
        put_u8(&mut colr, *count as u8);
        put_u32(&mut colr, u32::from(*first));
    }
    let layer_list_offset = colr.len();
    put_u32(&mut colr, u32::try_from(layers.len()).ok()?);
    let layer_offsets_start = colr.len();
    colr.resize(layer_offsets_start + layers.len() * 4, 0);
    for (index, (glyph, palette, gradient, alpha, nested, transform)) in
        paint_layers.iter().enumerate()
    {
        let paint_offset = colr.len().checked_sub(layer_list_offset)?;
        let offset_position = layer_offsets_start + index * 4;
        colr[offset_position..offset_position + 4]
            .copy_from_slice(&u32::try_from(paint_offset).ok()?.to_be_bytes());
        if let Some(transform) = transform {
            put_u8(&mut colr, 12); // PaintTransform
                                   // PaintTransform contains two Offset24 fields followed by the
                                   // 24-byte Affine2x3 record. The transform record starts after
                                   // the 7-byte PaintTransform header; the child paint starts
                                   // after that record.
            colr.extend_from_slice(&[0, 0, 31]); // child PaintGlyph Offset24
            colr.extend_from_slice(&[0, 0, 7]); // Affine2x3 Offset24
            for value in [
                transform.xx,
                transform.yx,
                transform.xy,
                transform.yy,
                transform.dx,
                transform.dy,
            ] {
                put_i32(&mut colr, checked_fixed_16_16(value, "COLR変形").ok()?);
            }
        }
        if *nested {
            put_u8(&mut colr, 11); // PaintColrGlyph
            put_u16(&mut colr, *glyph);
        } else {
            put_u8(&mut colr, 10); // PaintGlyph
            let child_offset = 6_u32;
            colr.extend_from_slice(&[0, 0, child_offset as u8]); // child Offset24
            put_u16(&mut colr, *glyph);
        }
        if !*nested {
            if let Some(gradient) = gradient {
                let (paint_format, color_line_offset) = match gradient.kind {
                    crate::font_data::ColorGradientKind::Linear => (4, 16),
                    crate::font_data::ColorGradientKind::Radial => (6, 16),
                    crate::font_data::ColorGradientKind::Sweep => (8, 12),
                };
                put_u8(&mut colr, paint_format);
                colr.extend_from_slice(&[0, 0, color_line_offset]); // ColorLine Offset24
                match gradient.kind {
                    crate::font_data::ColorGradientKind::Linear => {
                        for coordinate in [
                            gradient.x0,
                            gradient.y0,
                            gradient.x1,
                            gradient.y1,
                            gradient.x2,
                            gradient.y2,
                        ] {
                            put_i16(
                                &mut colr,
                                checked_i16(coordinate, "COLRグラデーション座標").ok()?,
                            );
                        }
                    }
                    crate::font_data::ColorGradientKind::Radial => {
                        put_i16(
                            &mut colr,
                            checked_i16(gradient.x0, "COLR円形グラデーションX").ok()?,
                        );
                        put_i16(
                            &mut colr,
                            checked_i16(gradient.y0, "COLR円形グラデーションY").ok()?,
                        );
                        put_u16(
                            &mut colr,
                            checked_u16(gradient.radius0, "COLR円形グラデーション半径").ok()?,
                        );
                        put_i16(
                            &mut colr,
                            checked_i16(gradient.x1, "COLR円形グラデーションX").ok()?,
                        );
                        put_i16(
                            &mut colr,
                            checked_i16(gradient.y1, "COLR円形グラデーションY").ok()?,
                        );
                        put_u16(
                            &mut colr,
                            checked_u16(gradient.radius1, "COLR円形グラデーション半径").ok()?,
                        );
                    }
                    crate::font_data::ColorGradientKind::Sweep => {
                        put_i16(
                            &mut colr,
                            checked_i16(gradient.x0, "COLRスイープグラデーションX").ok()?,
                        );
                        put_i16(
                            &mut colr,
                            checked_i16(gradient.y0, "COLRスイープグラデーションY").ok()?,
                        );
                        put_u16(&mut colr, gradient_angle(gradient.start_angle));
                        put_u16(&mut colr, gradient_angle(gradient.end_angle));
                    }
                }
                put_u8(
                    &mut colr,
                    match gradient.extend {
                        crate::font_data::ColorGradientExtend::Pad => 0,
                        crate::font_data::ColorGradientExtend::Repeat => 1,
                        crate::font_data::ColorGradientExtend::Reflect => 2,
                    },
                );
                let mut stops = gradient.effective_stops();
                if stops.is_empty() || stops.len() > u16::MAX as usize {
                    return None;
                }
                stops.sort_by(|left, right| {
                    left.offset
                        .partial_cmp(&right.offset)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                put_u16(&mut colr, u16::try_from(stops.len()).ok()?);
                for stop in stops {
                    put_u16(&mut colr, gradient_stop_offset(stop.offset));
                    put_u16(&mut colr, stop.palette_index);
                    put_u16(&mut colr, gradient_alpha(stop.alpha));
                }
            } else {
                put_u8(&mut colr, 2); // PaintSolid
                put_u16(&mut colr, *palette);
                // Solid alpha is an F2Dot14 value in COLR v1.
                put_u16(&mut colr, gradient_alpha(*alpha));
            }
        }
    }
    for index in 0..bases.len() {
        let paint_offset = paint_colr_layers_offset
            .checked_sub(base_glyph_list_offset)?
            .checked_add(index.checked_mul(6)?)?;
        let offset_position = base_glyph_list_offset + 4 + index * 6 + 2;
        colr[offset_position..offset_position + 4]
            .copy_from_slice(&u32::try_from(paint_offset).ok()?.to_be_bytes());
    }
    colr[base_glyph_list_offset_position..base_glyph_list_offset_position + 4]
        .copy_from_slice(&u32::try_from(base_glyph_list_offset).ok()?.to_be_bytes());
    colr[base_glyph_list_offset_position + 4..base_glyph_list_offset_position + 8]
        .copy_from_slice(&u32::try_from(layer_list_offset).ok()?.to_be_bytes());

    let entries = project.color_palettes.first()?.len();
    let palettes = project.color_palettes.len();
    if entries == 0 || entries > u16::MAX as usize || palettes > u16::MAX as usize {
        return None;
    }
    let records = entries.checked_mul(palettes)?;
    let has_palette_labels = project
        .color_palette_names
        .iter()
        .take(palettes)
        .any(|label| !label.trim().is_empty());
    let has_palette_types = project
        .color_palette_types
        .iter()
        .take(palettes)
        .any(|palette_type| *palette_type != 0);
    let has_palette_entry_labels = project
        .color_palette_entry_names
        .iter()
        .take(entries)
        .any(|label| !label.trim().is_empty());
    let use_cpal_v1 = has_palette_labels || has_palette_types || has_palette_entry_labels;
    let mut cpal = Vec::new();
    let records_offset = if use_cpal_v1 {
        let color_record_indices_offset = 12usize;
        let version_one_header_offset =
            color_record_indices_offset.checked_add(palettes.checked_mul(2)?)?;
        let types_offset = version_one_header_offset.checked_add(12)?;
        let labels_offset = types_offset.checked_add(palettes.checked_mul(4)?)?;
        let color_labels_offset = labels_offset.checked_add(palettes.checked_mul(2)?)?;
        let records_offset = (color_labels_offset + entries.checked_mul(2)? + 3) & !3;
        put_u16(&mut cpal, 1);
        put_u16(&mut cpal, u16::try_from(entries).ok()?);
        put_u16(&mut cpal, u16::try_from(palettes).ok()?);
        put_u16(&mut cpal, u16::try_from(records).ok()?);
        put_u32(&mut cpal, u32::try_from(records_offset).ok()?);
        for palette_index in 0..palettes {
            put_u16(
                &mut cpal,
                u16::try_from(palette_index.checked_mul(entries)?).ok()?,
            );
        }
        put_u32(&mut cpal, u32::try_from(types_offset).ok()?);
        put_u32(&mut cpal, u32::try_from(labels_offset).ok()?);
        put_u32(&mut cpal, u32::try_from(color_labels_offset).ok()?);
        for palette_index in 0..palettes {
            put_u32(
                &mut cpal,
                project
                    .color_palette_types
                    .get(palette_index)
                    .copied()
                    .unwrap_or(0),
            );
        }
        for palette_index in 0..palettes {
            let name_id = project
                .color_palette_names
                .get(palette_index)
                .filter(|label| !label.trim().is_empty())
                .and_then(|_| u16::try_from(1000usize.saturating_add(palette_index)).ok())
                .unwrap_or(u16::MAX);
            put_u16(&mut cpal, name_id);
        }
        for entry_index in 0..entries {
            let name_id = project
                .color_palette_entry_names
                .get(entry_index)
                .filter(|label| !label.trim().is_empty())
                .and_then(|_| u16::try_from(2000usize.saturating_add(entry_index)).ok())
                .unwrap_or(u16::MAX);
            put_u16(&mut cpal, name_id);
        }
        while cpal.len() < records_offset {
            put_u8(&mut cpal, 0);
        }
        records_offset
    } else {
        put_u16(&mut cpal, 0);
        put_u16(&mut cpal, u16::try_from(entries).ok()?);
        put_u16(&mut cpal, u16::try_from(palettes).ok()?);
        put_u16(&mut cpal, u16::try_from(records).ok()?);
        let records_offset = 12usize.checked_add(palettes.checked_mul(4)?)?;
        put_u32(&mut cpal, u32::try_from(records_offset).ok()?);
        for index in 0..palettes {
            put_u32(
                &mut cpal,
                u32::try_from(records_offset + index * entries * 4).ok()?,
            );
        }
        records_offset
    };
    for palette in &project.color_palettes {
        if palette.len() != entries {
            return None;
        }
        for &[r, g, b, a] in palette {
            cpal.extend_from_slice(&[b, g, r, a]);
        }
    }
    debug_assert_eq!(cpal.len(), records_offset + records * 4);
    Some((colr, cpal))
}
