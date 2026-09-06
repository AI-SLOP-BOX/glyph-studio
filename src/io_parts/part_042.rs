
pub fn load_ttf(path: &Path) -> Result<FontProject, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("TTF読み込みエラー: {e}"))?;
    let mut face =
        ttf_parser::Face::parse(&bytes, 0).map_err(|e| format!("フォント解析エラー: {e:?}"))?;
    let mut project = FontProject::new();
    project.preserved_tables = preserved_sfnt_tables(&bytes);
    for index in 0..face.names().len() {
        if let Some(name) = face.names().get(index) {
            if let Some(value) = name.to_string() {
                match name.name_id {
                    0 => project.metadata.copyright = value,
                    1 => project.metadata.family_name = value,
                    2 => project.metadata.style_name = value,
                    8 => project.metadata.manufacturer = value,
                    9 => project.metadata.designer = value,
                    5 => {
                        if let Some(version) = value
                            .strip_prefix("Version ")
                            .and_then(|value| value.trim().parse::<f64>().ok())
                        {
                            project.metadata.font_revision = version;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    project.metadata.units_per_em = face.units_per_em() as f64;
    project.metadata.ascender = face.ascender() as f64;
    project.metadata.descender = face.descender() as f64;
    project.metadata.line_gap = face.line_gap() as f64;
    if let Ok(font) = FontRef::new(&bytes) {
        if let Ok(head) = font.head() {
            project.metadata.head_flags = head.flags().bits();
            project.metadata.head_mac_style = head.mac_style().bits();
            project.metadata.lowest_rec_ppem = head.lowest_rec_ppem();
            project.metadata.font_direction_hint = head.font_direction_hint();
        }
        if let Ok(post) = font.post() {
            project.metadata.italic_angle = post.italic_angle().to_f64();
            project.metadata.underline_position = i16::from(post.underline_position()) as f64;
            project.metadata.underline_thickness = i16::from(post.underline_thickness()) as f64;
            project.metadata.is_fixed_pitch = post.is_fixed_pitch() != 0;
        }
    }
    project.metadata.weight_class = face.weight().to_number();
    project.metadata.width_class = face.width().to_number();
    if let Ok(font) = FontRef::new(&bytes) {
        if let Ok(hhea) = font.hhea() {
            project.metadata.caret_slope_rise = hhea.caret_slope_rise();
            project.metadata.caret_slope_run = hhea.caret_slope_run();
            project.metadata.caret_offset = hhea.caret_offset();
        }
        if let Ok(os2) = font.os2() {
            project.metadata.x_height = os2.sx_height().unwrap_or(0) as f64;
            project.metadata.cap_height = os2.s_cap_height().unwrap_or(0) as f64;
            project.metadata.win_ascent = os2.us_win_ascent();
            project.metadata.win_descent = os2.us_win_descent();
            project.metadata.vendor_id = os2.ach_vend_id().to_string();
            project.metadata.fs_type = os2.fs_type();
            project.metadata.fs_selection = os2.fs_selection().bits();
            project.metadata.subscript_x_size = os2.y_subscript_x_size();
            project.metadata.subscript_y_size = os2.y_subscript_y_size();
            project.metadata.subscript_x_offset = os2.y_subscript_x_offset();
            project.metadata.subscript_y_offset = os2.y_subscript_y_offset();
            project.metadata.superscript_x_size = os2.y_superscript_x_size();
            project.metadata.superscript_y_size = os2.y_superscript_y_size();
            project.metadata.superscript_x_offset = os2.y_superscript_x_offset();
            project.metadata.superscript_y_offset = os2.y_superscript_y_offset();
            project.metadata.strikeout_size = os2.y_strikeout_size();
            project.metadata.strikeout_position = os2.y_strikeout_position();
            project.metadata.family_class = os2.s_family_class();
            project.metadata.lower_optical_point_size =
                os2.us_lower_optical_point_size().unwrap_or(0);
            project.metadata.upper_optical_point_size =
                os2.us_upper_optical_point_size().unwrap_or(0);
            if let Ok(panose) = os2.panose_10().try_into() {
                project.metadata.panose = panose;
            }
            project.metadata.default_char = os2.us_default_char().unwrap_or(0);
            project.metadata.break_char = os2.us_break_char().unwrap_or(0);
            project.metadata.max_context = os2.us_max_context().unwrap_or(0);
        }
    }
    if let Some(fvar) = face.tables().fvar {
        let axes = fvar
            .axes
            .into_iter()
            .map(|axis| (axis.tag.to_string(), axis.def_value as f64))
            .collect();
        if let Some(master) = project.masters.first_mut() {
            master.axes = axes;
        }
    }
    // Some variable fonts omit STAT but still provide localized axis labels
    // through fvar.axisNameID. Use those labels as a fallback.
    if let Ok(font) = FontRef::new(&bytes) {
        if let (Ok(fvar), Ok(names_table)) = (font.fvar(), font.name()) {
            let string_data = names_table.string_data();
            if let Ok(arrays) = fvar.axis_instance_arrays() {
                for axis in arrays.axes() {
                    let tag = axis.axis_tag().to_string();
                    project.axis_flags.insert(tag.clone(), axis.flags());
                    if project.axis_names.contains_key(&tag) {
                        continue;
                    }
                    let axis_name = names_table
                        .name_record()
                        .iter()
                        .find(|record| record.name_id() == axis.axis_name_id())
                        .and_then(|record| record.string(string_data).ok())
                        .map(|name| name.chars().collect::<String>())
                        .filter(|name| !name.trim().is_empty());
                    if let Some(name) = axis_name {
                        project.axis_names.insert(tag, name);
                    }
                }
            }
        }
    }
    // STAT is the authoritative source for human-readable axis labels in
    // many production variable fonts. Preserve those labels instead of
    // falling back to the four-character axis tags on import.
    if let Ok(font) = FontRef::new(&bytes) {
        if let (Ok(stat), Ok(names_table)) = (font.stat(), font.name()) {
            let string_data = names_table.string_data();
            for axis in stat.design_axes().into_iter().flatten() {
                let tag = axis.axis_tag().to_string();
                let axis_name = names_table
                    .name_record()
                    .iter()
                    .find(|record| record.name_id() == axis.axis_name_id())
                    .and_then(|record| record.string(string_data).ok())
                    .map(|name| name.chars().collect::<String>())
                    .filter(|name| !name.trim().is_empty());
                if let Some(name) = axis_name {
                    project.axis_names.insert(tag, name);
                }
            }
        }
    }
    if let Some(fvar) = face.tables().fvar {
        let axis_tags = fvar
            .axes
            .into_iter()
            .map(|axis| axis.tag.to_string())
            .collect::<Vec<_>>();
        if let Ok(font) = FontRef::new(&bytes) {
            if let Ok(avar) = font.avar() {
                if avar.axis_count() as usize == axis_tags.len() {
                    for (tag, segment_maps) in axis_tags
                        .into_iter()
                        .zip(avar.axis_segment_maps().iter().filter_map(Result::ok))
                    {
                        let points = segment_maps
                            .axis_value_maps()
                            .iter()
                            .map(|point| crate::font_data::AxisMappingPoint {
                                input: point.from_coordinate().to_f32() as f64,
                                output: point.to_coordinate().to_f32() as f64,
                            })
                            .collect::<Vec<_>>();
                        if !points.is_empty()
                            && points
                                .iter()
                                .any(|point| (point.input - point.output).abs() > f64::EPSILON)
                        {
                            project.axis_mappings.insert(tag, points);
                        }
                    }
                }
            }
        }
    }
    let mut unicodes = std::collections::HashMap::<u16, Vec<u32>>::new();
    for codepoint in 0..=0x10FFFF_u32 {
        if let Some(character) = char::from_u32(codepoint) {
            if let Some(glyph_id) = face.glyph_index(character) {
                let mappings = unicodes.entry(glyph_id.0).or_default();
                if !mappings.contains(&codepoint) {
                    mappings.push(codepoint);
                }
            }
        }
    }
    for raw_id in 0..face.number_of_glyphs() {
        let glyph_id = ttf_parser::GlyphId(raw_id);
        let name = face
            .glyph_name(glyph_id)
            .map(str::to_string)
            .unwrap_or_else(|| format!("glyph{raw_id}"));
        let unicode = unicodes
            .get(&raw_id)
            .and_then(|values| values.first())
            .copied();
        let mut glyph = crate::font_data::GlyphData::new(name.clone(), unicode);
        if let Some(values) = unicodes.get(&raw_id) {
            glyph.unicodes = values.iter().copied().skip(1).collect();
        }
        glyph.width = face.glyph_hor_advance(glyph_id).unwrap_or(0) as f64;
        let mut collector = OutlineCollector::new();
        face.outline_glyph(glyph_id, &mut collector);
        if !collector.current.points.is_empty() {
            collector.contours.push(collector.current);
        }
        glyph.contours = collector.contours;
        project.glyphs.insert(name.clone(), glyph);
        project.glyph_order.push(name);
    }
    let names: Vec<String> = (0..face.number_of_glyphs())
        .map(|raw_id| {
            face.glyph_name(ttf_parser::GlyphId(raw_id))
                .map(str::to_string)
                .unwrap_or_else(|| format!("glyph{raw_id}"))
        })
        .collect();
    // Convert an imported GDEF glyph classification into an explicit
    // Feature File declaration. The exporter can then retain Ligature/Mark/
    // Component semantics even when the original font has no editable source.
    if let Ok(font) = FontRef::new(&bytes) {
        if let Ok(gdef) = font.gdef() {
            let mut gdef_source = String::new();
            let mut mark_set_source = String::new();
            if let Some(Ok(class_def)) = gdef.glyph_class_def() {
                let mut groups = [Vec::<String>::new(), Vec::new(), Vec::new(), Vec::new()];
                for (glyph_id, class) in class_def.iter() {
                    let Some(name) = names.get(glyph_id.to_u32() as usize) else {
                        continue;
                    };
                    let index = match class {
                        1..=4 => usize::from(class - 1),
                        _ => continue,
                    };
                    groups[index].push(name.clone());
                }
                if groups.iter().any(|group| !group.is_empty()) {
                    let definitions = groups
                        .iter()
                        .map(|group| format!("[{}]", group.join(" ")))
                        .collect::<Vec<_>>()
                        .join(", ");
                    gdef_source.push_str(&format!("GlyphClassDef {definitions}; "));
                }
            }
            if let Some(Ok(attach_list)) = gdef.attach_list() {
                let glyphs = attach_list.coverage().ok().map(|coverage| coverage.iter());
                if let Some(glyphs) = glyphs {
                    for (glyph_id, point_table) in glyphs.zip(attach_list.attach_points().iter()) {
                        let Ok(point_table) = point_table else {
                            continue;
                        };
                        let Some(name) = names.get(glyph_id.to_u32() as usize) else {
                            continue;
                        };
                        let points = point_table
                            .point_indices()
                            .iter()
                            .map(|point| point.get().to_string())
                            .collect::<Vec<_>>();
                        if !points.is_empty() {
                            gdef_source.push_str(&format!("Attach {name} {}; ", points.join(" ")));
                        }
                    }
                }
            }
            if let Some(Ok(lig_caret_list)) = gdef.lig_caret_list() {
                let glyphs = lig_caret_list
                    .coverage()
                    .ok()
                    .map(|coverage| coverage.iter());
                if let Some(glyphs) = glyphs {
                    for (glyph_id, lig_glyph) in glyphs.zip(lig_caret_list.lig_glyphs().iter()) {
                        let Ok(lig_glyph) = lig_glyph else {
                            continue;
                        };
                        let Some(name) = names.get(glyph_id.to_u32() as usize) else {
                            continue;
                        };
                        let values = lig_glyph
                            .caret_values()
                            .iter()
                            .filter_map(Result::ok)
                            .filter_map(|caret| match caret {
                                read_fonts::tables::gdef::CaretValue::Format1(value) => Some(
                                    format!("LigatureCaretByPos {name} {}", value.coordinate()),
                                ),
                                read_fonts::tables::gdef::CaretValue::Format2(value) => {
                                    Some(format!(
                                        "LigatureCaretByIndex {name} {}",
                                        value.caret_value_point_index()
                                    ))
                                }
                                read_fonts::tables::gdef::CaretValue::Format3(value)
                                    if value.device_offset().to_usize() == 0 =>
                                {
                                    Some(format!(
                                        "LigatureCaretByPos {name} {}",
                                        value.coordinate()
                                    ))
                                }
                                read_fonts::tables::gdef::CaretValue::Format3(_) => None,
                            })
                            .collect::<Vec<_>>();
                        if !values.is_empty() {
                            gdef_source.push_str(&format!("{}; ", values.join("; ")));
                        }
                    }
                }
            }
            if let Some(Ok(mark_sets)) = gdef.mark_glyph_sets_def() {
                for (index, coverage) in mark_sets.coverages().iter().enumerate() {
                    let Ok(coverage) = coverage else {
                        continue;
                    };
                    let glyphs = coverage
                        .iter()
                        .filter_map(|glyph_id| names.get(glyph_id.to_u32() as usize))
                        .cloned()
                        .collect::<Vec<_>>();
                    if !glyphs.is_empty() {
                        mark_set_source
                            .push_str(&format!("@GSMarkSet{index} = [{}]; ", glyphs.join(" ")));
                    }
                }
            }
            if !gdef_source.is_empty() {
                project.opentype_features =
                    format!("{mark_set_source}table GDEF {{ {gdef_source}}} GDEF;");
            } else if !mark_set_source.is_empty() {
                project.opentype_features = mark_set_source;
            }
        }
        if let Ok(name_table) = font.name() {
            let raw_name = font
                .table_data(read_fonts::types::Tag::new(b"name"))
                .map(|data| data.as_bytes());
            if let Some(raw_name) = raw_name {
                let storage_start = usize::from(name_table.storage_offset());
                let mut records = Vec::new();
                for record in name_table.name_record() {
                    let name_id = record.name_id().to_u16();
                    if name_id < 23 {
                        continue;
                    }
                    let start = storage_start + record.string_offset().to_usize();
                    let end = start + usize::from(record.length());
                    let Some(value) = decode_name_string(
                        record.platform_id(),
                        raw_name.get(start..end).unwrap_or_default(),
                    ) else {
                        continue;
                    };
                    if value.is_empty() {
                        continue;
                    }
                    records.push(format!(
                        "nameid {} {} {} 0x{:04X} \"{}\";",
                        name_id,
                        record.platform_id(),
                        record.encoding_id(),
                        record.language_id(),
                        escape_feature_name(&value)
                    ));
                }
                if !records.is_empty() {
                    if !project.opentype_features.is_empty() {
                        project.opentype_features.push('\n');
                    }
                    project.opentype_features.push_str("table name { ");
                    project.opentype_features.push_str(&records.join(" "));
                    project.opentype_features.push_str(" } name;");
                }
            }
        }
    }
    let imported_gsub = import_simple_gsub_features(&face, &names);
    if !imported_gsub.is_empty() {
        if !project.opentype_features.is_empty() {
            project.opentype_features.push('\n');
        }
        project.opentype_features.push_str(&imported_gsub);
    }
    if let Ok(font) = FontRef::new(&bytes) {
        let imported_contextual = imported_contextual_gsub_features(&font, &names);
        if !imported_contextual.is_empty() {
            if !project.opentype_features.is_empty() {
                project.opentype_features.push('\n');
            }
            project.opentype_features.push_str(&imported_contextual);
        }
    }
    let imported_gpos = import_simple_gpos_features(&face, &names);
    if !imported_gpos.is_empty() {
        if !project.opentype_features.is_empty() {
            project.opentype_features.push('\n');
        }
        project.opentype_features.push_str(&imported_gpos);
    }
    if let Ok(font) = FontRef::new(&bytes) {
        let imported_contextual = imported_contextual_gpos_features(&font, &names);
        if !imported_contextual.is_empty() {
            if !project.opentype_features.is_empty() {
                project.opentype_features.push('\n');
            }
            project.opentype_features.push_str(&imported_contextual);
        }
    }
    let mut imported_languagesystems = std::collections::BTreeSet::new();
    if let Some(gsub) = face.tables().gsub {
        collect_imported_languagesystems!(gsub, imported_languagesystems);
    }
    if let Some(gpos) = face.tables().gpos {
        collect_imported_languagesystems!(gpos, imported_languagesystems);
    }
    if !imported_languagesystems.is_empty() {
        let declarations = imported_languagesystems
            .into_iter()
            .collect::<Vec<_>>()
            .join(" ");
        project.opentype_features = if project.opentype_features.is_empty() {
            declarations
        } else {
            format!("{declarations}\n{}", project.opentype_features)
        };
    }
    import_mark_to_base_anchors(&face, &names, &mut project);
    import_cursive_anchors(&face, &names, &mut project);
    import_mark_to_mark_anchors(&face, &names, &mut project);
    import_mark_to_ligature_anchors(&face, &names, &mut project);
    // Import non-default Unicode Variation Sequences from cmap format 14.
    // Default UVS entries intentionally do not become project mappings: they
    // mean “use the ordinary cmap glyph”, not a separate glyph assignment.
    if let Ok(font) = FontRef::new(&bytes) {
        if let Ok(cmap) = font.cmap() {
            if let Some((_, uvs)) = cmap.uvs_subtable() {
                for (base, selector, variant) in uvs.iter() {
                    let read_fonts::tables::cmap::MapVariant::Variant(glyph_id) = variant else {
                        continue;
                    };
                    let Some(glyph) = names.get(glyph_id.to_u32() as usize) else {
                        continue;
                    };
                    if !project
                        .unicode_variation_sequences
                        .iter()
                        .any(|mapping| mapping.base == base && mapping.selector == selector)
                    {
                        project.unicode_variation_sequences.push(
                            crate::font_data::UnicodeVariationSequence {
                                base,
                                selector,
                                glyph: glyph.clone(),
                            },
                        );
                    }
                }
            }
        }
    }
    if let Ok(font) = FontRef::new(&bytes) {
        if let (Ok(colr), Ok(cpal)) = (font.colr(), font.cpal()) {
            if let Some(Ok(records)) = cpal.color_records_array() {
                let entries = usize::from(cpal.num_palette_entries());
                let palettes = usize::from(cpal.num_palettes());
                project.color_palettes = (0..palettes)
                    .map(|palette| {
                        (0..entries)
                            .filter_map(|index| {
                                records.get(palette * entries + index).map(|record| {
                                    [record.red(), record.green(), record.blue(), record.alpha()]
                                })
                            })
                            .collect()
                    })
                    .collect();
            }
            if cpal.version() >= 1 {
                if let Some(Ok(types)) = cpal.palette_types_array() {
                    project.color_palette_types = types
                        .iter()
                        .map(|palette_type| palette_type.get().bits())
                        .collect();
                }
                if let (Some(Ok(labels)), Ok(name_table)) =
                    (cpal.palette_entry_labels_array(), font.name())
                {
                    let string_data = name_table.string_data();
                    project.color_palette_entry_names = labels
                        .iter()
                        .map(|label| {
                            let name_id = label.get().to_u16();
                            if name_id == u16::MAX {
                                return String::new();
                            }
                            name_table
                                .name_record()
                                .iter()
                                .find(|record| record.name_id() == NameId::new(name_id))
                                .and_then(|record| record.string(string_data).ok())
                                .map(|name| name.chars().collect())
                                .unwrap_or_default()
                        })
                        .collect();
                }
                if let (Some(Ok(labels)), Ok(name_table)) =
                    (cpal.palette_labels_array(), font.name())
                {
                    let string_data = name_table.string_data();
                    project.color_palette_names = labels
                        .iter()
                        .map(|label| {
                            let name_id = label.get().to_u16();
                            if name_id == u16::MAX {
                                return String::new();
                            }
                            name_table
                                .name_record()
                                .iter()
                                .find(|record| record.name_id() == NameId::new(name_id))
                                .and_then(|record| record.string(string_data).ok())
                                .map(|name| name.chars().collect())
                                .unwrap_or_default()
                        })
                        .collect();
                }
            }
            for (raw_id, name) in names.iter().enumerate() {
                let Ok(raw_id) = u16::try_from(raw_id) else {
                    continue;
                };
                let Ok(Some(range)) = colr.v0_base_glyph(GlyphId::new(raw_id as u32)) else {
                    continue;
                };
                let layers = range
                    .filter_map(|index| colr.v0_layer(index).ok())
                    .filter_map(|(glyph, palette_index)| {
                        names.get(usize::from(glyph.to_u16())).map(|glyph| {
                            crate::font_data::ColorLayer {
                                glyph: glyph.clone(),
                                palette_index,
                                gradient: None,
                                alpha: 1.0,
                            }
                        })
                    })
                    .collect::<Vec<_>>();
                if !layers.is_empty() {
                    project.color_layers.insert(name.clone(), layers);
                }
            }
            // COLR v1 paints are flattened into the editable layer model for
            // PaintGlyph/Solid/gradient/transform and composite graphs. A
            // composite's blend mode is approximated by ordered SrcOver
            // layers because the project model does not yet store a general
            // paint graph.
            for (raw_id, name) in names.iter().enumerate() {
                let Ok(raw_id) = u16::try_from(raw_id) else {
                    continue;
                };
                let Ok(Some((paint, _))) = colr.v1_base_glyph(GlyphId::new(u32::from(raw_id)))
                else {
                    continue;
                };
                let mut imported_layers = Vec::new();
                let mut imported_transforms = Vec::new();
                import_colr_v1_paint(
                    &colr,
                    paint,
                    &names,
                    None,
                    None,
                    &mut imported_layers,
                    &mut imported_transforms,
                    0,
                );
                if !imported_layers.is_empty() {
                    project.color_layers.insert(name.clone(), imported_layers);
                    if imported_transforms.iter().any(Option::is_some) {
                        project
                            .color_layer_transforms
                            .insert(name.clone(), imported_transforms);
                    }
                }
            }
        }
        if let (Ok(vhea), Ok(vmtx)) = (font.vhea(), font.vmtx()) {
            project.metadata.vertical_caret_slope_rise = vhea.caret_slope_rise();
            project.metadata.vertical_caret_slope_run = vhea.caret_slope_run();
            project.metadata.vertical_caret_offset = vhea.caret_offset();
            let long_metrics = vmtx.v_metrics();
            let top_bearings = vmtx.top_side_bearings();
            let long_count = usize::from(vhea.number_of_long_ver_metrics());
            for (raw_id, name) in names.iter().enumerate() {
                let (advance, bearing) = if raw_id < long_metrics.len() {
                    let metric = long_metrics[raw_id];
                    (metric.advance(), metric.side_bearing())
                } else if let Some(metric) = long_metrics.last() {
                    let bearing = top_bearings
                        .get(raw_id.saturating_sub(long_count))
                        .map(|value| value.get())
                        .unwrap_or_else(|| metric.side_bearing());
                    (metric.advance(), bearing)
                } else {
                    continue;
                };
                project.vertical_metrics.insert(
                    name.clone(),
                    crate::font_data::VerticalMetrics {
                        advance_height: f64::from(advance),
                        top_side_bearing: f64::from(bearing),
                    },
                );
            }
        }
    }
    if let Some(kern) = face.tables().kern {
        for subtable in kern.subtables {
            if !subtable.horizontal || subtable.has_cross_stream || subtable.has_state_machine {
                continue;
            }
            for left in 0..face.number_of_glyphs() {
                for right in 0..face.number_of_glyphs() {
                    if let Some(value) = subtable
                        .glyphs_kerning(ttf_parser::GlyphId(left), ttf_parser::GlyphId(right))
                    {
                        if value != 0 {
                            project.kerning.insert(
                                (names[left as usize].clone(), names[right as usize].clone()),
                                value as f64,
                            );
                        }
                    }
                }
            }
        }
    }
    // Modern OpenType fonts commonly store kerning in GPOS rather than the
    // legacy `kern` table. Import explicit PairPos format 1 pairs as well.
    if let Some(gpos) = face.tables().gpos {
        for feature in gpos.features.into_iter() {
            if feature.tag != ttf_parser::Tag::from_bytes(b"kern") {
                continue;
            }
            for lookup_index in feature.lookup_indices.into_iter() {
                let Some(lookup) = gpos.lookups.get(lookup_index) else {
                    continue;
                };
                for subtable in lookup
                    .subtables
                    .into_iter::<ttf_parser::gpos::PositioningSubtable>()
                {
                    let ttf_parser::gpos::PositioningSubtable::Pair(
                        ttf_parser::gpos::PairAdjustment::Format1 { coverage, sets },
                    ) = subtable
                    else {
                        continue;
                    };
                    for left in 0..face.number_of_glyphs() {
                        let left_id = ttf_parser::GlyphId(left);
                        let Some(coverage_index) = coverage.get(left_id) else {
                            continue;
                        };
                        let Some(pair_set) = sets.get(coverage_index) else {
                            continue;
                        };
                        for right in 0..face.number_of_glyphs() {
                            let Some((first, _second)) = pair_set.get(ttf_parser::GlyphId(right))
                            else {
                                continue;
                            };
                            if first.x_advance != 0 {
                                project.kerning.insert(
                                    (names[left as usize].clone(), names[right as usize].clone()),
                                    first.x_advance as f64,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    // Also expand GPOS PairPos format 2 (class-based kerning). This is the
    // compact form used by many production fonts and is not exposed by the
    // high-level ttf-parser GPOS API.
    if let Ok(font) = FontRef::new(&bytes) {
        if let Ok(gpos) = font.gpos() {
            if let (Ok(features), Ok(lookups)) = (gpos.feature_list(), gpos.lookup_list()) {
                for record in features.feature_records() {
                    if record.feature_tag() != read_fonts::types::Tag::new(b"kern") {
                        continue;
                    }
                    let Ok(feature) = record.feature(features.offset_data()) else {
                        continue;
                    };
                    for lookup_index in feature.lookup_list_indices() {
                        let Ok(lookup) = lookups.lookups().get(lookup_index.get() as usize) else {
                            continue;
                        };
                        let Ok(PositionSubtables::Pair(subtables)) = lookup.subtables() else {
                            continue;
                        };
                        for subtable in subtables.iter().flatten() {
                            let PairPos::Format2(pair) = subtable else {
                                continue;
                            };
                            let Ok(coverage) = pair.coverage() else {
                                continue;
                            };
                            let Ok(class_def1) = pair.class_def1() else {
                                continue;
                            };
                            let Ok(class_def2) = pair.class_def2() else {
                                continue;
                            };
                            let context = ValueContext::default();
                            for left in 0..face.number_of_glyphs() {
                                let left_id = read_fonts::types::GlyphId::new(left.into());
                                if coverage.get(left_id).is_none() {
                                    continue;
                                }
                                let class1 = class_def1.get(left_id);
                                for right in 0..face.number_of_glyphs() {
                                    let right_id = read_fonts::types::GlyphId::new(right.into());
                                    let class2 = class_def2.get(right_id);
                                    let Ok(values) = pair.values(class1, class2, &context) else {
                                        continue;
                                    };
                                    if values[0].x_advance != 0 {
                                        project.kerning.insert(
                                            (
                                                names[left as usize].clone(),
                                                names[right as usize].clone(),
                                            ),
                                            values[0].x_advance as f64,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    if let Some(fvar) = face.tables().fvar {
        let axes: Vec<_> = fvar.axes.into_iter().collect();
        for axis in &axes {
            for (suffix, value) in [("min", axis.min_value), ("max", axis.max_value)] {
                let id = format!("{}-{}", axis.tag, suffix);
                if project.masters.iter().any(|master| master.id == id) {
                    continue;
                }
                for default_axis in &axes {
                    face.set_variation(default_axis.tag, default_axis.def_value);
                }
                face.set_variation(axis.tag, value);
                let mut master = FontMaster {
                    id: id.clone(),
                    name: id.clone(),
                    ..FontMaster::default()
                };
                for default_axis in &axes {
                    master
                        .axes
                        .insert(default_axis.tag.to_string(), default_axis.def_value as f64);
                }
                master.axes.insert(axis.tag.to_string(), value as f64);
                project.masters.push(master);
                for raw_id in 0..face.number_of_glyphs() {
                    let glyph_id = ttf_parser::GlyphId(raw_id);
                    let Some(name) = project.glyph_order.get(raw_id as usize) else {
                        continue;
                    };
                    let Some(glyph) = project.glyphs.get_mut(name) else {
                        continue;
                    };
                    let mut collector = OutlineCollector::new();
                    face.outline_glyph(glyph_id, &mut collector);
                    if !collector.current.points.is_empty() {
                        collector.contours.push(collector.current);
                    }
                    glyph.layers.insert(
                        id.clone(),
                        GlyphLayer {
                            width: face.glyph_hor_advance(glyph_id).unwrap_or(0) as f64,
                            contours: collector.contours,
                            components: Vec::new(),
                            anchors: Vec::new(),
                        },
                    );
                }
            }
        }
        if let Ok(font_ref) = FontRef::new(&bytes) {
            if let Ok(variable) = font_ref.fvar() {
                let names_table = font_ref.name().ok();
                let name_data = names_table.as_ref().map(|table| table.string_data());
                let instances = variable
                    .instances()
                    .ok()
                    .into_iter()
                    .flat_map(|items| items.iter().filter_map(Result::ok));
                for (instance_index, instance) in instances.enumerate() {
                    let instance_name = names_table
                        .as_ref()
                        .and_then(|table| {
                            table
                                .name_record()
                                .iter()
                                .find(|record| record.name_id() == instance.subfamily_name_id)
                        })
                        .and_then(|record| {
                            name_data
                                .as_ref()
                                .and_then(|data| record.string(*data).ok())
                        })
                        .map(|name| name.chars().collect::<String>())
                        .filter(|name| !name.trim().is_empty())
                        .unwrap_or_else(|| format!("Instance {}", instance_index + 1));
                    let mut instance_axes = std::collections::HashMap::new();
                    let mut instance_weight = 400.0;
                    let mut instance_width = 100.0;
                    for (axis, value) in axes.iter().zip(instance.coordinates.iter()) {
                        let value = value.get().to_f32() as f64;
                        let tag = axis.tag.to_string();
                        if tag.eq_ignore_ascii_case("wght") {
                            instance_weight = value;
                        } else if tag.eq_ignore_ascii_case("wdth") {
                            instance_width = value;
                        }
                        instance_axes.insert(tag, value);
                    }
                    project.instances.push(crate::font_data::FontInstance {
                        name: instance_name.clone(),
                        axes: instance_axes,
                        weight: instance_weight,
                        width: instance_width,
                    });
                    let id = format!("instance-{}", instance_index + 1);
                    if project.masters.iter().any(|master| master.id == id) {
                        continue;
                    }
                    for axis in &axes {
                        face.set_variation(axis.tag, axis.def_value);
                    }
                    let mut master = FontMaster {
                        id: id.clone(),
                        name: instance_name,
                        ..FontMaster::default()
                    };
                    for (axis, value) in axes.iter().zip(instance.coordinates.iter()) {
                        let value = value.get().to_f32();
                        face.set_variation(axis.tag, value);
                        master.axes.insert(axis.tag.to_string(), value as f64);
                    }
                    project.masters.push(master);
                    for raw_id in 0..face.number_of_glyphs() {
                        let glyph_id = ttf_parser::GlyphId(raw_id);
                        let Some(name) = project.glyph_order.get(raw_id as usize) else {
                            continue;
                        };
                        let Some(glyph) = project.glyphs.get_mut(name) else {
                            continue;
                        };
                        let mut collector = OutlineCollector::new();
                        face.outline_glyph(glyph_id, &mut collector);
                        if !collector.current.points.is_empty() {
                            collector.contours.push(collector.current);
                        }
                        glyph.layers.insert(
                            id.clone(),
                            GlyphLayer {
                                width: face.glyph_hor_advance(glyph_id).unwrap_or(0) as f64,
                                contours: collector.contours,
                                components: Vec::new(),
                                anchors: Vec::new(),
                            },
                        );
                    }
                }
            }
        }
        for axis in &axes {
            face.set_variation(axis.tag, axis.def_value);
        }
    }
    project.normalize_glyph_order();
    project.normalize_masters();
    project.preserved_layout_source = Some(project.feature_source());
    project.preserved_layout_fingerprint = Some(crate::export::layout_input_fingerprint(&project));
    Ok(project)
}
