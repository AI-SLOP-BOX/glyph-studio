
pub fn export_ttf(project: &FontProject, path: &Path) -> Result<(), String> {
    use fonttools::cmap::{cmap, CmapSubtable};
    use fonttools::font::{Font, SfntVersion, Table};
    use fonttools::fvar::{fvar, InstanceRecord, VariationAxisRecord};
    use fonttools::glyf::{glyf, Glyph};
    use fonttools::gvar::gvar;
    use fonttools::head::head;
    use fonttools::hhea::hhea;
    use fonttools::hmtx::{hmtx, Metric};
    use fonttools::maxp::maxp;
    use fonttools::name::{name, NameRecord, NameRecordID};
    use fonttools::os2::{os2, Panose};
    use fonttools::post::post;

    let validation_issues = validate_project(project);
    if !validation_issues.is_empty() {
        return Err(format!(
            "フォント検証に失敗しました: {}",
            validation_issues.join("; ")
        ));
    }

    let mut project = project.clone();
    // UFO/JSON files from older versions may contain an axis on only some
    // masters. Fill those coordinates from the default master before building
    // fvar/gvar so every exported instance has the same axis count.
    project.normalize_masters();
    let (conditional_substitutions, axis_bounds) =
        materialize_conditional_substitutions(&mut project);
    let export_master_id = project
        .masters
        .iter()
        .find(|master| master.id == project.default_master_id)
        .map(|master| master.id.clone())
        .or_else(|| project.masters.first().map(|master| master.id.clone()));
    if let Some(master_id) = export_master_id {
        if let Some(kerning) = project.kerning_by_master.get(&master_id).cloned() {
            project.kerning = kerning;
        }
        for glyph in project.glyphs.values_mut() {
            if let Some(layer) = glyph.layers.get(&master_id).cloned() {
                glyph.width = layer.width;
                glyph.contours = layer.contours;
                glyph.components = layer.components;
                glyph.anchors = layer.anchors;
            }
        }
    }
    if project.masters.len() >= 2 {
        flatten_variation_components(&mut project)?;
    }

    let upm = checked_u16(project.metadata.units_per_em, "UPM")?;
    let source_before_table_overrides = project.feature_source();
    apply_feature_table_overrides(&mut project, &source_before_table_overrides);
    let feature_source = project.feature_source();
    let preserve_imported_layout = project.preserved_layout_source.as_deref()
        == Some(feature_source.as_str())
        && project.preserved_layout_fingerprint == Some(layout_input_fingerprint(&project));
    let preserve_gsub = preserve_imported_layout && project.preserved_tables.contains_key("GSUB");
    let preserve_gpos = preserve_imported_layout && project.preserved_tables.contains_key("GPOS");
    let preserve_gdef = preserve_imported_layout && project.preserved_tables.contains_key("GDEF");
    let unicode_by_glyph = project
        .glyphs
        .iter()
        .filter_map(|(name, glyph)| {
            glyph
                .unicode
                .or_else(|| glyph.unicodes.first().copied())
                .map(|unicode| (name.clone(), unicode))
        })
        .collect::<BTreeMap<_, _>>();
    validate_feature_source(&feature_source)?;
    let base_master = project
        .masters
        .iter()
        .find(|master| master.id == project.default_master_id)
        .or_else(|| project.masters.first())
        .ok_or_else(|| "可変フォントには基準マスターが必要です".to_string())?;
    validate_master_axes(&project)?;
    let has_width_axis = project.masters.len() >= 2
        && project
            .masters
            .iter()
            .any(|master| (master.width - base_master.width).abs() > f64::EPSILON);
    if !(1..=1000).contains(&project.metadata.weight_class) {
        return Err("Weight Classは1〜1000で指定してください".into());
    }
    if !(1..=9).contains(&project.metadata.width_class) {
        return Err("Width Classは1〜9で指定してください".into());
    }
    if project.metadata.vendor_id.len() != 4 || !project.metadata.vendor_id.is_ascii() {
        return Err("Vendor IDはASCII 4文字で指定してください".into());
    }
    let names = project.glyph_names_sorted();
    let glyph_ids: std::collections::HashMap<&str, u16> = names
        .iter()
        .enumerate()
        .map(|(index, name)| (*name, (index + 1) as u16))
        .collect();
    let mut glyph_ids = glyph_ids;
    glyph_ids.insert(".notdef", 0);
    if names.len() >= u16::MAX as usize {
        return Err("グリフ数が多すぎます".into());
    }
    let empty = || Glyph {
        contours: vec![],
        components: vec![],
        overlap: false,
        xMin: 0,
        yMin: 0,
        xMax: 0,
        yMax: 0,
        instructions: vec![],
    };
    let mut glyphs = vec![empty()];
    let mut metrics = vec![Metric {
        advanceWidth: upm,
        lsb: 0,
    }];
    let mut mapping = BTreeMap::new();

    for (index, name) in names.iter().enumerate() {
        let source = project.glyphs.get(*name).unwrap();
        let mut output = empty();
        let mut contours = Vec::new();
        if source.components.is_empty() || !source.contours.is_empty() {
            append_contours(
                &project,
                &source.name,
                (1.0, 0.0, 0.0, 1.0, 0.0, 0.0),
                &mut Vec::new(),
                &mut contours,
            )?;
        } else {
            for component in &source.components {
                let glyph_index = *glyph_ids.get(component.base.as_str()).ok_or_else(|| {
                    format!("コンポーネント '{}' が見つかりません", component.base)
                })?;
                output.components.push(fonttools::glyf::Component {
                    glyph_index,
                    transformation: kurbo08::Affine::new([
                        component.x_scale,
                        component.yx_scale,
                        component.xy_scale,
                        component.y_scale,
                        component.x_offset,
                        component.y_offset,
                    ]),
                    match_points: None,
                    flags: fonttools::glyf::ComponentFlags::empty(),
                });
            }
        }
        output.contours = contours;
        let all = output.contours.iter().flatten();
        output.xMin = all.clone().map(|p| p.x).min().unwrap_or(0);
        output.xMax = all.clone().map(|p| p.x).max().unwrap_or(0);
        output.yMin = all.clone().map(|p| p.y).min().unwrap_or(0);
        output.yMax = all.map(|p| p.y).max().unwrap_or(0);
        if !output.components.is_empty() {
            if let Some((min_x, min_y, max_x, max_y)) =
                project.outline_bounds_for_glyph(&source.name)
            {
                output.xMin = checked_i16(min_x.floor(), "複合グリフX最小値")?;
                output.yMin = checked_i16(min_y.floor(), "複合グリフY最小値")?;
                output.xMax = checked_i16(max_x.ceil(), "複合グリフX最大値")?;
                output.yMax = checked_i16(max_y.ceil(), "複合グリフY最大値")?;
            }
        }
        metrics.push(Metric {
            advanceWidth: checked_u16(source.width, "グリフ幅")?,
            lsb: output.xMin,
        });
        let mut codepoints = source.unicodes.clone();
        if let Some(codepoint) = source.unicode {
            if !codepoints.contains(&codepoint) {
                codepoints.push(codepoint);
            }
        }
        for codepoint in codepoints {
            if (0xD800..=0xDFFF).contains(&codepoint) || codepoint > 0x10FFFF {
                return Err(format!(
                    "グリフ '{}' のUnicode U+{codepoint:04X}は不正です",
                    source.name
                ));
            }
            if mapping.insert(codepoint, (index + 1) as u16).is_some() {
                return Err(format!("Unicode U+{codepoint:04X}が重複しています"));
            }
        }
        glyphs.push(output);
    }

    let all = glyphs.iter().flat_map(|g| g.contours.iter().flatten());
    let xmin = all.clone().map(|p| p.x).min().unwrap_or(0);
    let xmax = all.clone().map(|p| p.x).max().unwrap_or(0);
    let ymin = all.clone().map(|p| p.y).min().unwrap_or(0);
    let ymax = all.map(|p| p.y).max().unwrap_or(0);
    let outline = glyf { glyphs };
    if mapping.is_empty() {
        return Err("TTF出力にはUnicodeを持つグリフが1つ以上必要です".into());
    }
    let stats = outline.maxp_statistics();
    let min_right_side_bearing = metrics
        .iter()
        .zip(&outline.glyphs)
        .map(|(metric, glyph)| metric.advanceWidth as i32 - metric.lsb as i32 - glyph.xMax as i32)
        .min()
        .unwrap_or(0);
    let min_right_side_bearing =
        checked_i16(min_right_side_bearing as f64, "最小右サイドベアリング")?;
    let number_of_h_metrics = metrics
        .iter()
        .rposition(|metric| metric.advanceWidth != metrics.last().unwrap().advanceWidth)
        .map(|index| index + 2)
        .unwrap_or(1) as u16;
    let mut font = Font::new(SfntVersion::TrueType);
    let mut head_table = head::new(
        project.metadata.font_revision as f32,
        upm,
        xmin,
        ymin,
        xmax,
        ymax,
    );
    if project.metadata.head_flags != 0 {
        head_table.flags = project.metadata.head_flags;
    }
    if project.metadata.lowest_rec_ppem != 0 {
        head_table.lowestRecPPEM = project.metadata.lowest_rec_ppem;
    }
    head_table.fontDirectionHint = project.metadata.font_direction_hint;
    head_table.macStyle = if project.metadata.head_mac_style != 0 {
        project.metadata.head_mac_style
    } else {
        mac_style_flags(&project.metadata)
    };
    font.tables.insert(*b"head", Table::Head(head_table));
    let master_metrics = project.master_metrics_for(&base_master.id);
    let ascender = checked_i16(master_metrics.ascender, "Ascender")?;
    let descender = checked_i16(master_metrics.descender, "Descender")?;
    let line_gap = checked_i16(master_metrics.line_gap, "Line Gap")?;
    let win_ascent = u16::try_from(i32::from(ascender).max(i32::from(ymax)).max(0))
        .map_err(|_| "WinAscentが範囲外です".to_string())?;
    let win_descent = u16::try_from((-i32::from(descender)).max(-i32::from(ymin)).max(0))
        .map_err(|_| "WinDescentが範囲外です".to_string())?;
    let first_char = mapping
        .keys()
        .copied()
        .filter(|codepoint| *codepoint <= 0xFFFF)
        .min()
        .unwrap_or(0) as u16;
    let last_char = mapping
        .keys()
        .copied()
        .filter(|codepoint| *codepoint <= 0xFFFF)
        .max()
        .unwrap_or(0) as u16;
    let os2_scale = upm.min(i16::MAX as u16) as i16;
    let (unicode_range1, unicode_range2, unicode_range3, unicode_range4) =
        unicode_range_bits(&mapping);
    let (code_page_range1, code_page_range2) = code_page_range_bits(&mapping);
    let average_width = if metrics.len() > 1 {
        let total: i64 = metrics
            .iter()
            .skip(1)
            .map(|metric| i64::from(metric.advanceWidth))
            .sum();
        checked_i16(total as f64 / (metrics.len() - 1) as f64, "平均字幅")?
    } else {
        checked_i16(upm as f64, "平均字幅")?
    };
    font.tables.insert(
        *b"OS/2",
        Table::Os2(os2 {
            version: if project.metadata.x_height != 0.0
                || project.metadata.cap_height != 0.0
                || project.metadata.default_char != 0
                || project.metadata.break_char != 0
                || project.metadata.max_context != 0
            {
                if project.metadata.lower_optical_point_size != 0
                    || project.metadata.upper_optical_point_size != 0
                {
                    5
                } else {
                    2
                }
            } else if project.metadata.lower_optical_point_size != 0
                || project.metadata.upper_optical_point_size != 0
            {
                5
            } else {
                0
            },
            xAvgCharWidth: average_width,
            usWeightClass: project.metadata.weight_class,
            usWidthClass: project.metadata.width_class,
            fsType: project.metadata.fs_type,
            ySubscriptXSize: if project.metadata.subscript_x_size != 0 {
                project.metadata.subscript_x_size
            } else {
                (os2_scale / 2).max(1)
            },
            ySubscriptYSize: if project.metadata.subscript_y_size != 0 {
                project.metadata.subscript_y_size
            } else {
                (os2_scale / 2).max(1)
            },
            ySubscriptXOffset: project.metadata.subscript_x_offset,
            ySubscriptYOffset: project.metadata.subscript_y_offset,
            ySuperscriptXSize: if project.metadata.superscript_x_size != 0 {
                project.metadata.superscript_x_size
            } else {
                (os2_scale / 2).max(1)
            },
            ySuperscriptYSize: if project.metadata.superscript_y_size != 0 {
                project.metadata.superscript_y_size
            } else {
                (os2_scale / 2).max(1)
            },
            ySuperscriptXOffset: project.metadata.superscript_x_offset,
            ySuperscriptYOffset: if project.metadata.superscript_y_offset != 0 {
                project.metadata.superscript_y_offset
            } else {
                (os2_scale / 2).max(1)
            },
            yStrikeoutSize: if project.metadata.strikeout_size != 0 {
                project.metadata.strikeout_size
            } else {
                (os2_scale / 20).max(1)
            },
            yStrikeoutPosition: if project.metadata.strikeout_position != 0 {
                project.metadata.strikeout_position
            } else {
                (os2_scale / 3).max(1)
            },
            sFamilyClass: project.metadata.family_class,
            panose: Panose {
                panose0: project.metadata.panose[0],
                panose1: project.metadata.panose[1],
                panose2: project.metadata.panose[2],
                panose3: project.metadata.panose[3],
                panose4: project.metadata.panose[4],
                panose5: project.metadata.panose[5],
                panose6: project.metadata.panose[6],
                panose7: project.metadata.panose[7],
                panose8: project.metadata.panose[8],
                panose9: project.metadata.panose[9],
            },
            ulUnicodeRange1: unicode_range1,
            ulUnicodeRange2: unicode_range2,
            ulUnicodeRange3: unicode_range3,
            ulUnicodeRange4: unicode_range4,
            achVendID: font_vendor_id(&project.metadata.vendor_id),
            fsSelection: os2_selection_flags(&project.metadata),
            usFirstCharIndex: first_char,
            usLastCharIndex: last_char,
            sTypoAscender: ascender,
            sTypoDescender: descender,
            sTypoLineGap: line_gap,
            usWinAscent: if project.metadata.win_ascent != 0 {
                project.metadata.win_ascent
            } else {
                win_ascent
            },
            usWinDescent: if project.metadata.win_descent != 0 {
                project.metadata.win_descent
            } else {
                win_descent
            },
            ulCodePageRange1: Some(code_page_range1),
            ulCodePageRange2: Some(code_page_range2),
            sxHeight: (project.metadata.x_height != 0.0)
                .then(|| checked_i16(project.metadata.x_height, "x-height"))
                .transpose()?,
            sCapHeight: (project.metadata.cap_height != 0.0)
                .then(|| checked_i16(project.metadata.cap_height, "Cap height"))
                .transpose()?,
            usDefaultChar: Some(if project.metadata.default_char != 0 {
                project.metadata.default_char
            } else {
                0
            }),
            usBreakChar: Some(if project.metadata.break_char != 0 {
                project.metadata.break_char
            } else if mapping.contains_key(&0x20) {
                0x20
            } else {
                0
            }),
            usMaxContext: Some(if project.metadata.max_context != 0 {
                project.metadata.max_context
            } else {
                max_feature_context(&feature_source)
            }),
            usLowerOpticalPointSize: (project.metadata.lower_optical_point_size != 0)
                .then_some(project.metadata.lower_optical_point_size),
            usUpperOpticalPointSize: (project.metadata.upper_optical_point_size != 0)
                .then_some(project.metadata.upper_optical_point_size),
        }),
    );
    font.tables.insert(
        *b"hhea",
        Table::Hhea(hhea {
            majorVersion: 1,
            minorVersion: 0,
            ascender,
            descender,
            lineGap: line_gap,
            advanceWidthMax: metrics.iter().map(|m| m.advanceWidth).max().unwrap_or(upm),
            minLeftSideBearing: metrics.iter().map(|m| m.lsb).min().unwrap_or(0),
            minRightSideBearing: min_right_side_bearing,
            xMaxExtent: xmax,
            caretSlopeRise: if project.metadata.caret_slope_rise != 0 {
                project.metadata.caret_slope_rise
            } else {
                1
            },
            caretSlopeRun: project.metadata.caret_slope_run,
            caretOffset: project.metadata.caret_offset,
            reserved0: 0,
            reserved1: 0,
            reserved2: 0,
            reserved3: 0,
            metricDataFormat: 0,
            numberOfHMetrics: number_of_h_metrics,
        }),
    );
    font.tables.insert(*b"glyf", Table::Glyf(outline));
    if project.masters.len() >= 2 {
        let first_id = &base_master.id;
        validate_component_master_topology(&project, first_id)?;
        validate_component_master_transforms(&project, first_id)?;
        let mut variations = vec![None];
        let mut has_variation = false;
        for name in &names {
            let source = project.glyphs.get(*name).unwrap();
            let variation = build_gvar_variation(
                source,
                &project,
                first_id,
                has_width_axis,
                &mut has_variation,
            )?;
            variations.push(variation);
        }
        if has_variation {
            let glyph_table = font.tables.get(b"glyf").and_then(|table| match table {
                Table::Glyf(glyphs) => Some(glyphs),
                _ => None,
            });
            let bytes = gvar { variations }.to_bytes(glyph_table);
            font.tables.insert(*b"gvar", Table::Unknown(bytes));
        }
    }
    font.tables.insert(
        *b"maxp",
        Table::Maxp(maxp::new10(
            stats.0, stats.1, stats.2, stats.3, stats.4, stats.5, stats.6,
        )),
    );
    let hmtx_bytes = hmtx { metrics }.to_bytes().0;
    font.tables.insert(*b"hmtx", Table::Unknown(hmtx_bytes));
    let (vhea_bytes, vmtx_bytes) =
        build_vertical_metrics_tables(&project, &names, &project.default_master_id, upm)?;
    font.tables.insert(*b"vhea", Table::Unknown(vhea_bytes));
    font.tables.insert(*b"vmtx", Table::Unknown(vmtx_bytes));
    if let Some((colr, cpal)) = build_color_tables(&project, &glyph_ids) {
        font.tables.insert(*b"COLR", Table::Unknown(colr));
        font.tables.insert(*b"CPAL", Table::Unknown(cpal));
    }
    if let Some(svg_table) = build_svg_table(&project, &glyph_ids) {
        font.tables.insert(*b"SVG ", Table::Unknown(svg_table));
    }
    let has_non_bmp = mapping.keys().any(|codepoint| *codepoint > 0xFFFF);
    if has_non_bmp || !project.unicode_variation_sequences.is_empty() {
        font.tables.insert(
            *b"cmap",
            Table::Unknown(build_cmap_with_variations(
                &mapping,
                &project.unicode_variation_sequences,
                &glyph_ids,
            )),
        );
    } else {
        font.tables.insert(
            *b"cmap",
            Table::Cmap(cmap {
                subtables: vec![
                    CmapSubtable {
                        format: 4,
                        platformID: 0,
                        encodingID: 3,
                        languageID: 0,
                        mapping: mapping.clone(),
                    },
                    CmapSubtable {
                        format: 4,
                        platformID: 3,
                        encodingID: 1,
                        languageID: 0,
                        mapping,
                    },
                ],
            }),
        );
    }
    if !preserve_gsub {
        if let Some(gsub_bytes) = build_simple_gsub_with_variations_and_unicode(
            &feature_source,
            &glyph_ids,
            &conditional_substitutions,
            &axis_bounds,
            &unicode_by_glyph,
        ) {
            font.tables.insert(*b"GSUB", Table::Unknown(gsub_bytes));
        }
    }
    if !preserve_gpos {
        if let Some(gpos_bytes) = build_kerning_gpos_with_unicode(
            &project,
            &glyph_ids,
            &feature_source,
            &unicode_by_glyph,
        ) {
            font.tables.insert(*b"GPOS", Table::Unknown(gpos_bytes));
        }
    }
    if !preserve_gdef {
        if let Some(gdef_bytes) = build_gdef(&project, &glyph_ids, &feature_source) {
            font.tables.insert(*b"GDEF", Table::Unknown(gdef_bytes));
        }
    }
    if !project.preserved_tables.contains_key("BASE") {
        if let Some(base_bytes) = build_base_table() {
            font.tables.insert(*b"BASE", Table::Unknown(base_bytes));
        }
    }
    let mut pairs = project
        .kerning
        .iter()
        .filter_map(|((left, right), value)| {
            Some((
                *glyph_ids.get(left.as_str())?,
                *glyph_ids.get(right.as_str())?,
                checked_i16(*value, "カーニング値").ok()?,
            ))
        })
        .collect::<Vec<_>>();
    pairs.sort_unstable_by_key(|(left, right, _)| (*left, *right));
    if !pairs.is_empty() {
        let n_pairs =
            u16::try_from(pairs.len()).map_err(|_| "カーニングペアが多すぎます".to_string())?;
        let max_power = 1_u16 << (15 - n_pairs.leading_zeros());
        let search_range = max_power * 6;
        let entry_selector = (15 - max_power.leading_zeros()) as u16;
        let range_shift = n_pairs * 6 - search_range;
        let subtable_length = 14_u16 + n_pairs * 6;
        let mut bytes = Vec::with_capacity(18 + pairs.len() * 6);
        bytes.extend_from_slice(&0_u16.to_be_bytes());
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(&0_u16.to_be_bytes());
        bytes.extend_from_slice(&subtable_length.to_be_bytes());
        bytes.extend_from_slice(&1_u16.to_be_bytes());
        bytes.extend_from_slice(&n_pairs.to_be_bytes());
        bytes.extend_from_slice(&search_range.to_be_bytes());
        bytes.extend_from_slice(&entry_selector.to_be_bytes());
        bytes.extend_from_slice(&range_shift.to_be_bytes());
        for (left, right, value) in pairs {
            bytes.extend_from_slice(&left.to_be_bytes());
            bytes.extend_from_slice(&right.to_be_bytes());
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        font.tables.insert(*b"kern", Table::Unknown(bytes));
    }
    font.tables.insert(
        *b"name",
        Table::Name(name {
            records: vec![
                NameRecord::windows_unicode(
                    NameRecordID::FontFamilyName,
                    project.metadata.family_name.clone(),
                ),
                NameRecord::windows_unicode(
                    NameRecordID::FullFontName,
                    format!(
                        "{} {}",
                        project.metadata.family_name, project.metadata.style_name
                    ),
                ),
                NameRecord::windows_unicode(
                    NameRecordID::FontSubfamilyName,
                    project.metadata.style_name.clone(),
                ),
                NameRecord::windows_unicode(
                    NameRecordID::PreferredFamilyName,
                    project.metadata.family_name.clone(),
                ),
                NameRecord::windows_unicode(
                    NameRecordID::PreferredSubfamilyName,
                    project.metadata.style_name.clone(),
                ),
                NameRecord::windows_unicode(
                    3_u16,
                    format!(
                        "{};{:.3};{}",
                        project.metadata.family_name,
                        project.metadata.font_revision,
                        postscript_name(
                            &project.metadata.family_name,
                            &project.metadata.style_name
                        )
                    ),
                ),
                NameRecord::windows_unicode(16_u16, project.metadata.family_name.clone()),
                NameRecord::windows_unicode(17_u16, project.metadata.style_name.clone()),
                NameRecord::windows_unicode(21_u16, project.metadata.family_name.clone()),
                NameRecord::windows_unicode(22_u16, project.metadata.style_name.clone()),
                NameRecord::windows_unicode(
                    NameRecordID::Version,
                    format!("Version {:.3}", project.metadata.font_revision),
                ),
                NameRecord::windows_unicode(
                    NameRecordID::PostscriptName,
                    postscript_name(&project.metadata.family_name, &project.metadata.style_name),
                ),
            ],
        }),
    );
    if let Some(Table::Name(names_table)) = font.tables.get_mut(b"name") {
        for (name_id, value) in [
            (NameRecordID::Copyright, &project.metadata.copyright),
            (NameRecordID::Designer, &project.metadata.designer),
            (NameRecordID::Manufacturer, &project.metadata.manufacturer),
        ] {
            if !value.trim().is_empty() {
                names_table
                    .records
                    .push(NameRecord::windows_unicode(name_id, value.clone()));
            }
        }
        // CPAL v1 palette labels use name IDs outside the standardized range.
        // Keep the IDs deterministic so a round trip does not depend on UI order.
        for (palette_index, label) in project
            .color_palette_names
            .iter()
            .enumerate()
            .take(project.color_palettes.len())
        {
            if let Ok(name_id) = u16::try_from(1000usize.saturating_add(palette_index)) {
                if !label.trim().is_empty() {
                    names_table
                        .records
                        .push(NameRecord::windows_unicode(name_id, label.clone()));
                }
            }
        }
        for (entry_index, label) in project.color_palette_entry_names.iter().enumerate() {
            if let Ok(name_id) = u16::try_from(2000usize.saturating_add(entry_index)) {
                if !label.trim().is_empty() {
                    names_table
                        .records
                        .push(NameRecord::windows_unicode(name_id, label.clone()));
                }
            }
        }
        for number in 1_u16..=20 {
            let ss_tag = format!("ss{number:02}");
            if feature_source.contains(&format!("feature {ss_tag}")) {
                let name_id = 499 + number;
                let records = feature_name_records(&feature_source, &ss_tag, name_id);
                if records.is_empty() {
                    names_table.records.push(NameRecord::windows_unicode(
                        name_id,
                        format!("Stylistic Set {number}"),
                    ));
                } else {
                    names_table.records.extend(records);
                }
            }
            let cv_tag = format!("cv{number:02}");
            if feature_source.contains(&format!("feature {cv_tag}")) {
                let name_id = 519 + number;
                let records = feature_name_records(&feature_source, &cv_tag, name_id);
                if records.is_empty() {
                    names_table.records.push(NameRecord::windows_unicode(
                        name_id,
                        format!("Character Variant {number}"),
                    ));
                } else {
                    names_table.records.extend(records);
                }
            }
        }
        for override_record in parse_feature_name_records(&feature_source) {
            names_table.records.retain(|record| {
                (
                    record.platformID,
                    record.encodingID,
                    record.languageID,
                    record.nameID,
                ) != (
                    override_record.platformID,
                    override_record.encodingID,
                    override_record.languageID,
                    override_record.nameID,
                )
            });
            names_table.records.push(override_record);
        }
    }
    if project.masters.len() >= 2 {
        let mut custom_axis_tags: Vec<String> = project
            .masters
            .iter()
            .flat_map(|master| master.axes.keys())
            .filter(|tag| tag.len() == 4 && tag.is_ascii())
            .cloned()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        custom_axis_tags.retain(|tag| {
            let mut values = project
                .masters
                .iter()
                .map(|master| master.axes.get(tag).copied().unwrap_or(0.0));
            values
                .next()
                .is_some_and(|first| values.any(|value| (value - first).abs() > f64::EPSILON))
        });
        let implicit_width_axis =
            has_width_axis && !custom_axis_tags.iter().any(|tag| tag == "wdth");
        if let Some(Table::Name(names_table)) = font.tables.get_mut(b"name") {
            if custom_axis_tags.is_empty() {
                names_table.records.push(NameRecord::windows_unicode(
                    256_u16,
                    project
                        .axis_names
                        .get("wght")
                        .cloned()
                        .unwrap_or_else(|| "Weight".into()),
                ));
            } else {
                for (index, tag) in custom_axis_tags.iter().enumerate() {
                    names_table.records.push(NameRecord::windows_unicode(
                        256 + index as u16,
                        project
                            .axis_names
                            .get(tag)
                            .cloned()
                            .unwrap_or_else(|| tag.clone()),
                    ));
                }
            }
            if has_width_axis && !custom_axis_tags.iter().any(|tag| tag == "wdth") {
                names_table.records.push(NameRecord::windows_unicode(
                    if custom_axis_tags.is_empty() {
                        257
                    } else {
                        256 + custom_axis_tags.len() as u16
                    },
                    project
                        .axis_names
                        .get("wdth")
                        .cloned()
                        .unwrap_or_else(|| "Width".into()),
                ));
            }
            for (index, master) in project.masters.iter().enumerate() {
                names_table.records.push(NameRecord::windows_unicode(
                    300_u16 + index as u16,
                    master.name.clone(),
                ));
            }
            for (index, instance) in project.instances.iter().enumerate() {
                names_table.records.push(NameRecord::windows_unicode(
                    400_u16 + index as u16,
                    instance.name.clone(),
                ));
            }
        }
        let axis_value =
            |master: &FontMaster, tag: &str| master.axes.get(tag).copied().unwrap_or(0.0);
        let instance_axis_value =
            |instance: &FontInstance, tag: &str| instance.axes.get(tag).copied().unwrap_or(0.0);
        let mut axes: Vec<VariationAxisRecord> = custom_axis_tags
            .iter()
            .enumerate()
            .map(|(index, tag)| VariationAxisRecord {
                axisTag: tag.as_bytes().try_into().unwrap(),
                flags: project.axis_flags.get(tag).copied().unwrap_or(0),
                minValue: project
                    .masters
                    .iter()
                    .map(|m| axis_value(m, tag))
                    .fold(f64::INFINITY, f64::min) as f32,
                defaultValue: axis_value(base_master, tag) as f32,
                maxValue: project
                    .masters
                    .iter()
                    .map(|m| axis_value(m, tag))
                    .fold(f64::NEG_INFINITY, f64::max) as f32,
                axisNameID: 256 + index as u16,
            })
            .collect();
        if axes.is_empty() {
            axes.push(VariationAxisRecord {
                axisTag: *b"wght",
                flags: project.axis_flags.get("wght").copied().unwrap_or(0),
                minValue: project
                    .masters
                    .iter()
                    .map(|m| m.weight)
                    .fold(f64::INFINITY, f64::min) as f32,
                defaultValue: base_master.weight as f32,
                maxValue: project
                    .masters
                    .iter()
                    .map(|m| m.weight)
                    .fold(f64::NEG_INFINITY, f64::max) as f32,
                axisNameID: 256,
            });
        }
        if has_width_axis && !custom_axis_tags.iter().any(|tag| tag == "wdth") {
            let min_width = project
                .masters
                .iter()
                .map(|master| master.width)
                .fold(f64::INFINITY, f64::min) as f32;
            let max_width = project
                .masters
                .iter()
                .map(|master| master.width)
                .fold(f64::NEG_INFINITY, f64::max) as f32;
            axes.push(VariationAxisRecord {
                axisTag: *b"wdth",
                flags: project.axis_flags.get("wdth").copied().unwrap_or(0),
                minValue: min_width,
                defaultValue: base_master.width as f32,
                maxValue: max_width,
                axisNameID: if custom_axis_tags.is_empty() {
                    257
                } else {
                    256 + custom_axis_tags.len() as u16
                },
            });
        }
        let hvar_axis_tags = axes
            .iter()
            .map(|axis| String::from_utf8_lossy(&axis.axisTag).to_string())
            .collect::<Vec<_>>();
        font.tables.insert(
            *b"fvar",
            Table::Fvar(fvar {
                axes,
                instances: if project.instances.is_empty() {
                    project
                        .masters
                        .iter()
                        .enumerate()
                        .map(|(index, master)| InstanceRecord {
                            subfamilyNameID: 300 + index as u16,
                            coordinates: custom_axis_tags
                                .iter()
                                .map(|tag| axis_value(master, tag) as f32)
                                .chain(if custom_axis_tags.is_empty() {
                                    Some(master.weight as f32)
                                } else {
                                    None
                                })
                                .chain(implicit_width_axis.then_some(master.width as f32))
                                .collect(),
                            postscriptNameID: None,
                        })
                        .collect()
                } else {
                    project
                        .instances
                        .iter()
                        .enumerate()
                        .map(|(index, instance)| InstanceRecord {
                            subfamilyNameID: 400 + index as u16,
                            coordinates: custom_axis_tags
                                .iter()
                                .map(|tag| instance_axis_value(instance, tag) as f32)
                                .chain(if custom_axis_tags.is_empty() {
                                    Some(instance.weight as f32)
                                } else {
                                    None
                                })
                                .chain(implicit_width_axis.then_some(instance.width as f32))
                                .collect(),
                            postscriptNameID: None,
                        })
                        .collect()
                },
            }),
        );
        if let Some(avar_bytes) = build_avar(&hvar_axis_tags, &project.axis_mappings) {
            font.tables.insert(*b"avar", Table::Unknown(avar_bytes));
        }
        if let Some(hvar_bytes) = build_hvar(&project, &names, base_master, &hvar_axis_tags) {
            font.tables.insert(*b"HVAR", Table::Unknown(hvar_bytes));
        }
        if let Some(vvar_bytes) = build_vvar(&project, &names, base_master, &hvar_axis_tags) {
            font.tables.insert(*b"VVAR", Table::Unknown(vvar_bytes));
        }
        if let Some(mvar_bytes) = build_mvar(&project, base_master, &hvar_axis_tags) {
            font.tables.insert(*b"MVAR", Table::Unknown(mvar_bytes));
        }
        let mut stat_axes = custom_axis_tags
            .iter()
            .enumerate()
            .map(|(index, tag)| (tag.as_bytes().try_into().unwrap(), 256 + index as u16))
            .collect::<Vec<([u8; 4], u16)>>();
        if custom_axis_tags.is_empty() {
            stat_axes.push((*b"wght", 256));
        }
        if implicit_width_axis {
            stat_axes.push((
                *b"wdth",
                if custom_axis_tags.is_empty() {
                    257
                } else {
                    256 + custom_axis_tags.len() as u16
                },
            ));
        }
        let stat_values = if project.instances.is_empty() {
            project
                .masters
                .iter()
                .map(|master| {
                    custom_axis_tags
                        .iter()
                        .map(|tag| axis_value(master, tag) as f32)
                        .chain(if custom_axis_tags.is_empty() {
                            Some(master.weight as f32)
                        } else {
                            None
                        })
                        .chain(implicit_width_axis.then_some(master.width as f32))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        } else {
            project
                .instances
                .iter()
                .map(|instance| {
                    custom_axis_tags
                        .iter()
                        .map(|tag| instance_axis_value(instance, tag) as f32)
                        .chain(if custom_axis_tags.is_empty() {
                            Some(instance.weight as f32)
                        } else {
                            None
                        })
                        .chain(implicit_width_axis.then_some(instance.width as f32))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        };
        let stat_name_ids = if project.instances.is_empty() {
            project
                .masters
                .iter()
                .enumerate()
                .map(|(index, _)| 300_u16 + index as u16)
                .collect::<Vec<_>>()
        } else {
            project
                .instances
                .iter()
                .enumerate()
                .map(|(index, _)| 400_u16 + index as u16)
                .collect::<Vec<_>>()
        };
        font.tables.insert(
            *b"STAT",
            Table::Unknown(build_stat_table_with_values(
                &stat_axes,
                &stat_values,
                &stat_name_ids,
            )),
        );
    }
    font.tables.insert(
        *b"post",
        Table::Post(post::new(
            2.0,
            project.metadata.italic_angle as f32,
            checked_i16(project.metadata.underline_position, "Underline position")?,
            checked_i16(project.metadata.underline_thickness, "Underline thickness")?,
            project.metadata.is_fixed_pitch,
            Some(
                std::iter::once(".notdef")
                    .chain(names.iter().copied())
                    .map(str::to_string)
                    .collect(),
            ),
        )),
    );
    // Advertise the standard bitmap behavior for all ppem sizes. This keeps
    // rasterizers from applying legacy embedded-bitmap rules to outline fonts.
    font.tables.insert(
        *b"gasp",
        Table::Unknown({
            let mut bytes = Vec::with_capacity(8);
            bytes.extend_from_slice(&1u16.to_be_bytes());
            bytes.extend_from_slice(&1u16.to_be_bytes());
            bytes.extend_from_slice(&0xFFFFu16.to_be_bytes());
            bytes.extend_from_slice(&0x000Fu16.to_be_bytes());
            bytes
        }),
    );
    // Preserve tables not yet modelled by Glyph Studio. Generated tables above
    // always win, so editing outlines/metrics/features cannot leave stale
    // copies of core tables in the output while specialised tables such as
    // MATH, JSTF, bitmap strikes, AAT, meta, and DSIG remain available.
    for (tag, bytes) in &project.preserved_tables {
        let Ok(tag_bytes) = <[u8; 4]>::try_from(tag.as_bytes()) else {
            continue;
        };
        font.tables
            .entry(tag_bytes)
            .or_insert_with(|| Table::Unknown(bytes.clone()));
    }
    let mut file = File::create(path).map_err(|e| format!("TTF作成エラー: {e}"))?;
    font.save(&mut file);
    Ok(())
}
