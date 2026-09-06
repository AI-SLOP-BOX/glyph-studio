#[allow(clippy::type_complexity)]
struct ParsedPositionFeatures {
    raw_feature_blocks: Vec<(Tag, String)>,
    lookup_mark_sets: BTreeMap<Tag, String>,
    mark_sets: BTreeMap<String, (u16, layout::CoverageTable)>,
    lookup_flags: BTreeMap<Tag, layout::LookupFlag>,
    single_positions: Vec<(Tag, GlyphId16, gpos::ValueRecord)>,
    pair_positions: Vec<(Tag, GlyphId16, GlyphId16, gpos::ValueRecord, gpos::ValueRecord)>,
    contextual_positions: Vec<(Tag, Vec<GlyphId16>, usize, gpos::ValueRecord)>,
    chained_positions: Vec<(Tag, Vec<GlyphId16>, Vec<GlyphId16>, GlyphId16, Vec<GlyphId16>, gpos::ValueRecord)>,
    ignored_positions: Vec<(Tag, Vec<Vec<GlyphId16>>)>,
}

fn parse_position_features(source: &str, glyph_ids: &std::collections::HashMap<&str, u16>) -> ParsedPositionFeatures {
    let expanded_source = expand_named_feature_lookups(&expand_named_feature_classes(source));
    let raw_feature_blocks = extract_feature_blocks(&expanded_source);
    let lookup_mark_sets = extract_feature_blocks(source)
        .iter()
        .filter_map(|(tag, block)| parse_lookup_mark_filtering_set(block).map(|name| (*tag, name)))
        .collect::<BTreeMap<_, _>>();
    let mark_sets = parse_mark_glyph_sets(source, glyph_ids);
    let feature_blocks = raw_feature_blocks.clone();
    let lookup_flags = feature_blocks.iter().map(|(tag, block)| (*tag, parse_lookup_flags(block))).collect::<BTreeMap<_, _>>();
    let mut single_positions = Vec::<(Tag, GlyphId16, gpos::ValueRecord)>::new();
    let mut pair_positions = Vec::<(Tag, GlyphId16, GlyphId16, gpos::ValueRecord, gpos::ValueRecord)>::new();
    let mut contextual_positions = Vec::<(Tag, Vec<GlyphId16>, usize, gpos::ValueRecord)>::new();
    let mut chained_positions = Vec::<(Tag, Vec<GlyphId16>, Vec<GlyphId16>, GlyphId16, Vec<GlyphId16>, gpos::ValueRecord)>::new();
    let mut ignored_positions = Vec::<(Tag, Vec<Vec<GlyphId16>>)>::new();
    for (feature_tag, block) in feature_blocks {
        for statement in block.split(';') {
            let tokens: Vec<_> = statement.split_whitespace().collect();
            if tokens.first() == Some(&"ignore") && tokens.get(1) == Some(&"pos") {
                if let Some(sequence) = parse_feature_sequence(&tokens[2..], glyph_ids) {
                    ignored_positions.push((feature_tag, sequence));
                }
                continue;
            }
            let Some(pos_index) = tokens.iter().position(|token| *token == "pos") else {
                continue;
            };
            let tokens = &tokens[pos_index + 1..];
            let shorthand_value = tokens.last().and_then(|token| token.parse::<i16>().ok());
            let Some(value_start) = tokens
                .iter()
                .position(|token| token.starts_with('<'))
                .or_else(|| shorthand_value.map(|_| tokens.len().saturating_sub(1)))
            else {
                continue;
            };
            if value_start == 0 {
                continue;
            }
            let glyph_tokens = &tokens[..value_start];
            let mut operands = Vec::<Vec<&str>>::new();
            let mut operand = Vec::new();
            let mut bracket_depth = 0_i32;
            for token in glyph_tokens {
                bracket_depth += token.matches('[').count() as i32;
                bracket_depth -= token.matches(']').count() as i32;
                operand.push(*token);
                if bracket_depth == 0 {
                    operands.push(std::mem::take(&mut operand));
                }
            }
            if bracket_depth != 0 {
                continue;
            }
            let value_records = if tokens[value_start].starts_with('<') {
                let value_text = tokens[value_start..].join(" ");
                parse_feature_value_records(&value_text)
            } else {
                shorthand_value
                    .map(|value| ParsedGposValueRecord {
                        values: vec![0, 0, value, 0],
                        ..Default::default()
                    })
                    .into_iter()
                    .collect::<Vec<_>>()
            };
            let parse_value = |parsed: &ParsedGposValueRecord| {
                if !(1..=4).contains(&parsed.values.len()) {
                    return None;
                }
                let mut format = gpos::ValueFormat::empty();
                let mut record = gpos::ValueRecord::new();
                if let Some(&value) = parsed.values.first() {
                    format |= gpos::ValueFormat::X_PLACEMENT;
                    record = record.with_x_placement(value);
                }
                if let Some(&value) = parsed.values.get(1) {
                    format |= gpos::ValueFormat::Y_PLACEMENT;
                    record = record.with_y_placement(value);
                }
                if let Some(&value) = parsed.values.get(2) {
                    format |= gpos::ValueFormat::X_ADVANCE;
                    record = record.with_x_advance(value);
                }
                if let Some(&value) = parsed.values.get(3) {
                    format |= gpos::ValueFormat::Y_ADVANCE;
                    record = record.with_y_advance(value);
                }
                if let Some(device) = parsed.devices[0].clone() {
                    format |= gpos::ValueFormat::X_PLACEMENT_DEVICE;
                    record = record.with_x_placement_device(device);
                }
                if let Some(device) = parsed.devices[1].clone() {
                    format |= gpos::ValueFormat::Y_PLACEMENT_DEVICE;
                    record = record.with_y_placement_device(device);
                }
                if let Some(device) = parsed.devices[2].clone() {
                    format |= gpos::ValueFormat::X_ADVANCE_DEVICE;
                    record = record.with_x_advance_device(device);
                }
                if let Some(device) = parsed.devices[3].clone() {
                    format |= gpos::ValueFormat::Y_ADVANCE_DEVICE;
                    record = record.with_y_advance_device(device);
                }
                Some(record.with_explicit_value_format(format))
            };
            if glyph_tokens.iter().any(|token| token.ends_with('\'')) {
                let Some(value) = value_records.first().and_then(&parse_value) else {
                    continue;
                };
                for (sequence, target_index, _) in parse_context_sequences(glyph_tokens, glyph_ids) {
                    if target_index == 0 {
                        contextual_positions.push((feature_tag, sequence, target_index, value.clone()));
                    } else if target_index < sequence.len() {
                        chained_positions.push((
                            feature_tag,
                            sequence[..target_index].iter().rev().copied().collect(),
                            Vec::new(),
                            sequence[target_index],
                            sequence[target_index + 1..].to_vec(),
                            value.clone(),
                        ));
                    }
                }
                continue;
            }
            let expand = |tokens: &[&str]| {
                clean_feature_class(tokens)
                    .into_iter()
                    .filter_map(|name| glyph_ids.get(name.as_str()).copied())
                    .map(GlyphId16::new)
                    .collect::<Vec<_>>()
            };
            if operands.len() == 1 {
                let glyphs = expand(&operands[0]);
                let Some(value) = value_records.first().and_then(&parse_value) else {
                    continue;
                };
                single_positions.extend(glyphs.into_iter().map(|glyph| (feature_tag, glyph, value.clone())));
            } else if operands.len() == 2 {
                let left = expand(&operands[0]);
                let right = expand(&operands[1]);
                let Some(first) = value_records.first().and_then(&parse_value) else {
                    continue;
                };
                let second = value_records.get(1).and_then(parse_value).unwrap_or_else(gpos::ValueRecord::new);
                for left_glyph in &left {
                    for right_glyph in &right {
                        pair_positions.push((feature_tag, *left_glyph, *right_glyph, first.clone(), second.clone()));
                    }
                }
            }
        }
    }
    ParsedPositionFeatures {
        raw_feature_blocks,
        lookup_mark_sets,
        mark_sets,
        lookup_flags,
        single_positions,
        pair_positions,
        contextual_positions,
        chained_positions,
        ignored_positions,
    }
}
