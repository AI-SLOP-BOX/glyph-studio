
fn apply_feature_table_overrides(project: &mut FontProject, source: &str) {
    let mut hhea_values = BTreeMap::<String, f64>::new();
    let mut vertical_values = Vec::<(String, String, f64)>::new();
    let mut post_italic_angle = project.metadata.italic_angle;
    let mut post_underline_position = project.metadata.underline_position;
    let mut post_underline_thickness = project.metadata.underline_thickness;
    let mut post_is_fixed_pitch = project.metadata.is_fixed_pitch;
    for (tag, body) in extract_table_blocks(source) {
        for statement in body.split(';') {
            let tokens = statement.split_whitespace().collect::<Vec<_>>();
            if tokens.len() < 2 {
                continue;
            }
            let key = tokens[0].to_ascii_lowercase();
            if tag.eq_ignore_ascii_case("OS/2") && key == "vendor" {
                let Some(start) = statement.find('"').map(|index| index + 1) else {
                    continue;
                };
                let Some(end) = statement[start..].find('"').map(|index| start + index) else {
                    continue;
                };
                let vendor = &statement[start..end];
                if vendor.len() == 4 && vendor.is_ascii() {
                    project.metadata.vendor_id = vendor.to_string();
                }
                continue;
            }
            if tag.eq_ignore_ascii_case("post") && key == "isfixedpitch" {
                if let Some(raw_value) = tokens.get(1) {
                    post_is_fixed_pitch = matches!(
                        raw_value.trim_matches(|character: char| "<>()".contains(character)),
                        "1" | "true" | "yes"
                    );
                }
                continue;
            }
            if tag.eq_ignore_ascii_case("OS/2") && key == "panose" {
                let values = tokens
                    .iter()
                    .skip(1)
                    .take(10)
                    .filter_map(|raw| {
                        parse_feature_table_number(raw)
                            .and_then(|value| u8::try_from(value as i64).ok())
                    })
                    .collect::<Vec<_>>();
                if values.len() == 10 {
                    project.metadata.panose.copy_from_slice(&values);
                }
                continue;
            }
            let value_index = if tag.eq_ignore_ascii_case("vmtx") {
                2
            } else {
                1
            };
            let Some(raw_value) = tokens.get(value_index) else {
                continue;
            };
            let Some(value) = parse_feature_table_number(raw_value) else {
                continue;
            };
            if tag.eq_ignore_ascii_case("head") && key == "fontrevision" {
                if value.is_finite() && (0.0..=65535.0).contains(&value) {
                    project.metadata.font_revision = value;
                }
            } else if tag.eq_ignore_ascii_case("head") && key == "flags" {
                if value.is_finite() && (0.0..=u16::MAX as f64).contains(&value) {
                    project.metadata.head_flags = value as u16;
                }
            } else if tag.eq_ignore_ascii_case("head") && key == "macstyle" {
                if value.is_finite() && (0.0..=u16::MAX as f64).contains(&value) {
                    project.metadata.head_mac_style = value as u16;
                }
            } else if tag.eq_ignore_ascii_case("head") && key == "lowestrecppem" {
                if value.is_finite() && (0.0..=u16::MAX as f64).contains(&value) {
                    project.metadata.lowest_rec_ppem = value as u16;
                }
            } else if tag.eq_ignore_ascii_case("head") && key == "fontdirectionhint" {
                if value.is_finite() && (i16::MIN as f64..=i16::MAX as f64).contains(&value) {
                    project.metadata.font_direction_hint = value as i16;
                }
            } else if tag.eq_ignore_ascii_case("post") && key == "italicangle" {
                if value.is_finite() {
                    post_italic_angle = value;
                }
            } else if tag.eq_ignore_ascii_case("post") && key == "underlineposition" {
                if value.is_finite() {
                    post_underline_position = value;
                }
            } else if tag.eq_ignore_ascii_case("post") && key == "underlinethickness" {
                if value.is_finite() {
                    post_underline_thickness = value;
                }
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "weightclass"
                && value.is_finite()
                && (1.0..=1000.0).contains(&value)
            {
                project.metadata.weight_class = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "widthclass"
                && value.is_finite()
                && (1.0..=9.0).contains(&value)
            {
                project.metadata.width_class = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "fstype"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.fs_type = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "fsselection"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.fs_selection = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "defaultchar"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.default_char = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "breakchar"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.break_char = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "maxcontext"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.max_context = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && matches!(
                    key.as_str(),
                    "ysubscriptxsize"
                        | "ysubcriptysize"
                        | "ysubscriptxoffset"
                        | "ysubscriptyoffset"
                )
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                match key.as_str() {
                    "ysubscriptxsize" => project.metadata.subscript_x_size = value as i16,
                    "ysubscriptysize" => project.metadata.subscript_y_size = value as i16,
                    "ysubscriptxoffset" => project.metadata.subscript_x_offset = value as i16,
                    _ => project.metadata.subscript_y_offset = value as i16,
                }
            } else if tag.eq_ignore_ascii_case("OS/2")
                && matches!(
                    key.as_str(),
                    "ysuperscriptxsize"
                        | "ysuperscriptysize"
                        | "ysuperscriptxoffset"
                        | "ysuperscriptyoffset"
                )
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                match key.as_str() {
                    "ysuperscriptxsize" => project.metadata.superscript_x_size = value as i16,
                    "ysuperscriptysize" => project.metadata.superscript_y_size = value as i16,
                    "ysuperscriptxoffset" => project.metadata.superscript_x_offset = value as i16,
                    _ => project.metadata.superscript_y_offset = value as i16,
                }
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "ystrikeoutsize"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.strikeout_size = value as i16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "ystrikeoutposition"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.strikeout_position = value as i16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "sfamilyclass"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.family_class = value as i16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "loweropticalpointsize"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.lower_optical_point_size = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "upperopticalpointsize"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.upper_optical_point_size = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "winascent"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.win_ascent = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "windescent"
                && value.is_finite()
                && (0.0..=u16::MAX as f64).contains(&value)
            {
                project.metadata.win_descent = value as u16;
            } else if tag.eq_ignore_ascii_case("OS/2") && key == "xheight" && value.is_finite() {
                project.metadata.x_height = value;
            } else if tag.eq_ignore_ascii_case("OS/2") && key == "capheight" && value.is_finite() {
                project.metadata.cap_height = value;
            } else if tag.eq_ignore_ascii_case("OS/2") && key == "typoascender" && value.is_finite()
            {
                hhea_values.insert("ascender".into(), value);
            } else if tag.eq_ignore_ascii_case("OS/2")
                && key == "typodescender"
                && value.is_finite()
            {
                hhea_values.insert("descender".into(), value);
            } else if tag.eq_ignore_ascii_case("OS/2") && key == "typolinegap" && value.is_finite()
            {
                hhea_values.insert("linegap".into(), value);
            } else if tag.eq_ignore_ascii_case("hhea")
                && key == "caretsloperise"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.caret_slope_rise = value as i16;
            } else if tag.eq_ignore_ascii_case("hhea")
                && key == "caretsloperun"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.caret_slope_run = value as i16;
            } else if tag.eq_ignore_ascii_case("hhea")
                && key == "caretoffset"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.caret_offset = value as i16;
            } else if tag.eq_ignore_ascii_case("vhea")
                && key == "caretsloperise"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.vertical_caret_slope_rise = value as i16;
            } else if tag.eq_ignore_ascii_case("vhea")
                && key == "caretsloperun"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.vertical_caret_slope_run = value as i16;
            } else if tag.eq_ignore_ascii_case("vhea")
                && key == "caretoffset"
                && value.is_finite()
                && (i16::MIN as f64..=i16::MAX as f64).contains(&value)
            {
                project.metadata.vertical_caret_offset = value as i16;
            } else if tag.eq_ignore_ascii_case("vmtx")
                && matches!(key.as_str(), "vertoriginy" | "vertadvancey")
                && tokens.len() > 2
            {
                let glyphs = clean_feature_class(&tokens[1..value_index]);
                for glyph in glyphs {
                    vertical_values.push((key.clone(), glyph, value));
                }
            } else if tag.eq_ignore_ascii_case("hhea")
                && matches!(key.as_str(), "ascender" | "descender" | "linegap")
                && value.is_finite()
            {
                hhea_values.insert(key, value);
            }
        }
    }
    project.metadata.italic_angle = post_italic_angle;
    project.metadata.underline_position = post_underline_position;
    project.metadata.underline_thickness = post_underline_thickness;
    project.metadata.is_fixed_pitch = post_is_fixed_pitch;
    for (kind, glyph_name, value) in vertical_values {
        if !project.glyphs.contains_key(&glyph_name) {
            continue;
        }
        let max_y = project
            .outline_bounds_for_glyph(&glyph_name)
            .map(|(_, _, _, max_y)| max_y)
            .unwrap_or(0.0);
        let fallback = project.vertical_metrics_for_glyph(&glyph_name);
        let metric = project
            .vertical_metrics
            .entry(glyph_name.clone())
            .or_insert(fallback);
        if kind == "vertoriginy" {
            metric.top_side_bearing = value - max_y;
        } else {
            metric.advance_height = value;
        }
    }
    if hhea_values.is_empty() {
        return;
    }
    let default_metrics = crate::font_data::MasterMetrics {
        ascender: project.metadata.ascender,
        descender: project.metadata.descender,
        line_gap: project.metadata.line_gap,
    };
    if let Some(master_id) = project
        .masters
        .iter()
        .find(|master| master.id == project.default_master_id)
        .or_else(|| project.masters.first())
        .map(|master| master.id.clone())
    {
        let metrics = project
            .metrics_by_master
            .entry(master_id)
            .or_insert(default_metrics);
        if let Some(value) = hhea_values.get("ascender") {
            metrics.ascender = *value;
        }
        if let Some(value) = hhea_values.get("descender") {
            metrics.descender = *value;
        }
        if let Some(value) = hhea_values.get("linegap") {
            metrics.line_gap = *value;
        }
    } else {
        if let Some(value) = hhea_values.get("ascender") {
            project.metadata.ascender = *value;
        }
        if let Some(value) = hhea_values.get("descender") {
            project.metadata.descender = *value;
        }
        if let Some(value) = hhea_values.get("linegap") {
            project.metadata.line_gap = *value;
        }
    }
}
