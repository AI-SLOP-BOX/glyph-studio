#[rustfmt::skip]
fn build_kerning_gpos_with_unicode(project: &FontProject, glyph_ids: &std::collections::HashMap<&str, u16>, source: &str, unicode_by_glyph: &BTreeMap<String, u32>) -> Option<Vec<u8>> {
    let source = expand_named_anchors(&expand_named_value_records(&normalize_feature_keywords(source)));
    let (grouped, class_pairs, left_groups, right_groups) = collect_kerning_pairs(project, glyph_ids);
    let mut lookups = Vec::new();
    let mut feature_indices = Vec::<(Tag, u16)>::new();

    // Compile the broadly useful subset of Adobe feature-file positioning
    // syntax in addition to the editor's native kerning/anchor data. Keeping
    // these in separate lookups means hand-authored features can coexist with
    // the generated `kern`, `mark`, and `mkmk` features.
    let ParsedPositionFeatures {
        raw_feature_blocks,
        lookup_mark_sets,
        mark_sets,
        lookup_flags,
        mut single_positions,
        pair_positions,
        contextual_positions,
        chained_positions,
        ignored_positions,
    } = parse_position_features(&source, glyph_ids);
    single_positions.sort_by_key(|(_, glyph, _)| glyph.to_u16());
    for tag in feature_tags_from_positions(&single_positions) {
        let entries = single_positions.iter().filter(|(entry_tag, _, _)| *entry_tag == tag).collect::<Vec<_>>();
        if entries.is_empty() {
            continue;
        }
        let coverage: layout::CoverageTable = entries.iter().map(|(_, glyph, _)| *glyph).collect();
        let values = entries.iter().map(|(_, _, value)| (*value).clone()).collect();
        let lookup = layout::Lookup::new(lookup_flags.get(&tag).copied().unwrap_or_else(layout::LookupFlag::empty), vec![gpos::SinglePos::Format2(gpos::SinglePosFormat2::new(coverage, values))]);
        let lookup = apply_lookup_mark_set(lookup, tag, &lookup_mark_sets, &mark_sets);
        feature_indices.push((tag, lookups.len() as u16));
        lookups.push(gpos::PositionLookup::Single(lookup));
    }
    for tag in feature_tags_from_pair_positions(&pair_positions) {
        let entries = pair_positions.iter().filter(|(entry_tag, _, _, _, _)| *entry_tag == tag).collect::<Vec<_>>();
        let mut grouped = BTreeMap::<GlyphId16, Vec<_>>::new();
        for (_, left, right, first, second) in entries {
            grouped.entry(*left).or_default().push((*right, (*first).clone(), (*second).clone()));
        }
        if grouped.is_empty() {
            continue;
        }
        let coverage: layout::CoverageTable = grouped.keys().copied().collect();
        let pair_sets = grouped.into_values().map(|pairs| gpos::PairSet::new(pairs.into_iter().map(|(right, first, second)| gpos::PairValueRecord::new(right, first, second)).collect())).collect();
        let pair_pos = gpos::PairPos::format_1(coverage, pair_sets);
        let lookup = layout::Lookup::new(lookup_flags.get(&tag).copied().unwrap_or_else(layout::LookupFlag::empty), vec![pair_pos]);
        let lookup = apply_lookup_mark_set(lookup, tag, &lookup_mark_sets, &mark_sets);
        feature_indices.push((tag, lookups.len() as u16));
        lookups.push(gpos::PositionLookup::Pair(lookup));
    }
    for (feature_tag, sequence, target_index, value) in contextual_positions {
        if sequence.len() < 2 || target_index >= sequence.len() {
            continue;
        }
        let target = sequence[target_index];
        let single_lookup = layout::Lookup::new(lookup_flags.get(&feature_tag).copied().unwrap_or_else(layout::LookupFlag::empty), vec![gpos::SinglePos::Format2(gpos::SinglePosFormat2::new(std::iter::once(target).collect(), vec![value]))]);
        let single_lookup = apply_lookup_mark_set(single_lookup, feature_tag, &lookup_mark_sets, &mark_sets);
        let single_lookup_index = u16::try_from(lookups.len()).ok()?;
        lookups.push(gpos::PositionLookup::Single(single_lookup));
        let rule = layout::SequenceRule::new(sequence[1..].to_vec(), vec![layout::SequenceLookupRecord::new(u16::try_from(target_index).ok()?, single_lookup_index)]);
        let context = layout::Lookup::new(lookup_flags.get(&feature_tag).copied().unwrap_or_else(layout::LookupFlag::empty), vec![gpos::PositionSequenceContext::from(layout::SequenceContext::format_1(std::iter::once(sequence[0]).collect(), vec![Some(layout::SequenceRuleSet::new(vec![rule]))]))]);
        let context = apply_lookup_mark_set(context, feature_tag, &lookup_mark_sets, &mark_sets);
        feature_indices.push((feature_tag, lookups.len() as u16));
        lookups.push(gpos::PositionLookup::Contextual(context));
    }
    for (feature_tag, backtrack, input, target, lookahead, value) in chained_positions {
        let single_lookup = layout::Lookup::new(lookup_flags.get(&feature_tag).copied().unwrap_or_else(layout::LookupFlag::empty), vec![gpos::SinglePos::Format2(gpos::SinglePosFormat2::new(std::iter::once(target).collect(), vec![value]))]);
        let single_lookup = apply_lookup_mark_set(single_lookup, feature_tag, &lookup_mark_sets, &mark_sets);
        let single_lookup_index = u16::try_from(lookups.len()).ok()?;
        lookups.push(gpos::PositionLookup::Single(single_lookup));
        let rule = layout::ChainedSequenceRule::new(backtrack, input, lookahead, vec![layout::SequenceLookupRecord::new(0, single_lookup_index)]);
        let context = layout::Lookup::new(lookup_flags.get(&feature_tag).copied().unwrap_or_else(layout::LookupFlag::empty), vec![gpos::PositionChainContext::from(layout::ChainedSequenceContext::format_1(std::iter::once(target).collect(), vec![Some(layout::ChainedSequenceRuleSet::new(vec![rule]))]))]);
        let context = apply_lookup_mark_set(context, feature_tag, &lookup_mark_sets, &mark_sets);
        feature_indices.push((feature_tag, lookups.len() as u16));
        lookups.push(gpos::PositionLookup::ChainContextual(context));
    }
    for (feature_tag, sequence) in ignored_positions {
        let context = layout::Lookup::new(lookup_flags.get(&feature_tag).copied().unwrap_or_else(layout::LookupFlag::empty), vec![gpos::PositionChainContext::from(layout::ChainedSequenceContext::format_3(Vec::new(), sequence.into_iter().map(Into::into).collect(), Vec::new(), Vec::new()))]);
        let context = apply_lookup_mark_set(context, feature_tag, &lookup_mark_sets, &mark_sets);
        feature_indices.push((feature_tag, lookups.len() as u16));
        lookups.push(gpos::PositionLookup::ChainContextual(context));
    }
    let mut cursive_anchors = BTreeMap::<GlyphId16, (Option<gpos::AnchorTable>, Option<gpos::AnchorTable>)>::new();
    for name in project.glyphs.keys() {
        let Some(&glyph_id) = glyph_ids.get(name.as_str()) else {
            continue;
        };
        for anchor in project.anchors_for_glyph(name) {
            let anchor_kind = anchor.name.trim_start_matches('_');
            if anchor_kind != "entry" && anchor_kind != "exit" {
                continue;
            }
            let (Ok(x), Ok(y)) = (checked_i16(anchor.x, "カ―シブアンカーX"), checked_i16(anchor.y, "カ―シブアンカーY")) else {
                continue;
            };
            let anchors = cursive_anchors.entry(GlyphId16::new(glyph_id)).or_default();
            let value = Some(gpos::AnchorTable::format_1(x, y));
            if anchor_kind == "entry" {
                anchors.0 = value;
            } else {
                anchors.1 = value;
            }
        }
    }
    let mut cursive_feature_tag = Tag::new(b"curs");
    for (feature_tag, block) in &raw_feature_blocks {
        for statement in block.split(';') {
            let tokens = statement.split_whitespace().collect::<Vec<_>>();
            if tokens.first() != Some(&"pos") || tokens.get(1) != Some(&"cursive") {
                continue;
            }
            let operand_indices = tokens.iter().enumerate().filter_map(|(index, token)| (*token == "<anchor" || *token == "NULL").then_some(index)).collect::<Vec<_>>();
            let Some(&entry_index) = operand_indices.first() else {
                continue;
            };
            let Some(&exit_index) = operand_indices.get(1) else {
                continue;
            };
            let parse_anchor = |index: usize| {
                if tokens.get(index) == Some(&"NULL") {
                    return Some(None);
                }
                let (x, y) = parse_feature_anchor(&tokens, index)?;
                Some(Some(gpos::AnchorTable::format_1(x, y)))
            };
            let (Some(entry), Some(exit)) = (parse_anchor(entry_index), parse_anchor(exit_index)) else {
                continue;
            };
            for glyph_name in clean_feature_class(&tokens[2..entry_index]) {
                let Some(&glyph_id) = glyph_ids.get(glyph_name.as_str()) else {
                    continue;
                };
                cursive_anchors.entry(GlyphId16::new(glyph_id)).or_default().clone_from(&(entry.clone(), exit.clone()));
            }
            cursive_feature_tag = *feature_tag;
        }
    }
    if !cursive_anchors.is_empty() {
        let coverage: layout::CoverageTable = cursive_anchors.keys().copied().collect();
        let records = cursive_anchors.into_values().map(|(entry, exit)| gpos::EntryExitRecord::new(entry, exit)).collect();
        let cursive = gpos::CursivePosFormat1::new(coverage, records);
        let lookup = layout::Lookup::new(lookup_flags.get(&cursive_feature_tag).copied().unwrap_or_else(layout::LookupFlag::empty), vec![cursive]);
        let lookup = apply_lookup_mark_set(lookup, cursive_feature_tag, &lookup_mark_sets, &mark_sets);
        feature_indices.push((cursive_feature_tag, lookups.len() as u16));
        lookups.push(gpos::PositionLookup::Cursive(lookup));
    }
    if !class_pairs.is_empty() {
        let mut left_class_ids = std::collections::BTreeMap::<String, u16>::new();
        let mut right_class_ids = std::collections::BTreeMap::<String, u16>::new();
        for (left_group, right_group) in class_pairs.keys() {
            let next_left = left_class_ids.len() as u16 + 1;
            left_class_ids.entry(left_group.clone()).or_insert(next_left);
            let next_right = right_class_ids.len() as u16 + 1;
            right_class_ids.entry(right_group.clone()).or_insert(next_right);
        }
        let class_def1 = layout::ClassDef::from_iter(left_groups.iter().flat_map(|(group, names)| {
            let class = left_class_ids[*group];
            names.iter().filter_map(move |name| glyph_ids.get(*name).copied().map(|id| (GlyphId16::new(id), class)))
        }));
        let class_def2 = layout::ClassDef::from_iter(right_groups.iter().flat_map(|(group, names)| {
            let class = right_class_ids[*group];
            names.iter().filter_map(move |name| glyph_ids.get(*name).copied().map(|id| (GlyphId16::new(id), class)))
        }));
        let mut rows = vec![gpos::Class1Record::new(vec![gpos::Class2Record::new(gpos::ValueRecord::new().with_explicit_value_format(gpos::ValueFormat::X_ADVANCE), gpos::ValueRecord::new(),); right_class_ids.len() + 1]); left_class_ids.len() + 1];
        for ((left_group, right_group), value) in class_pairs {
            let left_class = left_class_ids[&left_group] as usize;
            let right_class = right_class_ids[&right_group] as usize;
            rows[left_class].class2_records[right_class] = gpos::Class2Record::new(gpos::ValueRecord::new().with_x_advance(value), gpos::ValueRecord::new());
        }
        let coverage: layout::CoverageTable = class_def1.iter().map(|(glyph, _)| glyph).collect();
        let pair_pos = gpos::PairPos::format_2(coverage, class_def1, class_def2, rows);
        let lookup = layout::Lookup::new(layout::LookupFlag::empty(), vec![pair_pos]);
        feature_indices.push((Tag::new(b"kern"), lookups.len() as u16));
        lookups.push(gpos::PositionLookup::Pair(lookup));
    }
    if !grouped.is_empty() {
        let coverage: layout::CoverageTable = grouped.keys().copied().collect();
        let pair_sets = grouped
            .into_values()
            .map(|pairs| {
                let mut pairs = pairs;
                pairs.sort_by_key(|(right, _, direct)| (right.to_u16(), !*direct));
                pairs.dedup_by_key(|(right, _, _)| *right);
                gpos::PairSet::new(pairs.into_iter().map(|(right, value, _)| gpos::PairValueRecord::new(right, gpos::ValueRecord::new().with_x_advance(value), gpos::ValueRecord::new())).collect())
            })
            .collect();
        let pair_pos = gpos::PairPos::format_1(coverage, pair_sets);
        let lookup = layout::Lookup::new(layout::LookupFlag::empty(), vec![pair_pos]);
        feature_indices.push((Tag::new(b"kern"), lookups.len() as u16));
        lookups.push(gpos::PositionLookup::Pair(lookup));
    }
    let mut mark_names = BTreeMap::<GlyphId16, Vec<(String, gpos::AnchorTable)>>::new();
    let mut base_names = BTreeMap::<GlyphId16, Vec<(String, gpos::AnchorTable)>>::new();
    let mut source_mark_to_mark = false;
    for name in project.glyphs.keys() {
        let Some(&glyph_id) = glyph_ids.get(name.as_str()) else {
            continue;
        };
        for anchor in project.anchors_for_glyph(name) {
            let Ok(x) = checked_i16(anchor.x, "アンカーX") else {
                continue;
            };
            let Ok(y) = checked_i16(anchor.y, "アンカーY") else {
                continue;
            };
            if let Some(mark_name) = anchor.name.strip_prefix('_') {
                if !mark_name.is_empty() {
                    mark_names.entry(GlyphId16::new(glyph_id)).or_default().push((mark_name.to_string(), gpos::AnchorTable::format_1(x, y)));
                }
            } else if !anchor.name.is_empty() {
                base_names.entry(GlyphId16::new(glyph_id)).or_default().push((anchor.name.clone(), gpos::AnchorTable::format_1(x, y)));
            }
        }
    }
    for block in std::iter::once(source.as_str()).chain(raw_feature_blocks.iter().map(|(_, block)| block.as_str())) {
        for statement in block.split(';') {
            let tokens = statement.split_whitespace().collect::<Vec<_>>();
            if tokens.first() == Some(&"markClass") {
                let Some(anchor_index) = tokens.iter().position(|token| *token == "<anchor") else {
                    continue;
                };
                let Some((x, y)) = parse_feature_anchor(&tokens, anchor_index) else {
                    continue;
                };
                let Some(class_name) = tokens.last().map(|token| token.trim_start_matches('@')) else {
                    continue;
                };
                for glyph_name in clean_feature_class(&tokens[1..anchor_index]) {
                    let Some(&glyph_id) = glyph_ids.get(glyph_name.as_str()) else {
                        continue;
                    };
                    mark_names.entry(GlyphId16::new(glyph_id)).or_default().push((class_name.to_string(), gpos::AnchorTable::format_1(x, y)));
                }
            } else if tokens.first() == Some(&"pos") && tokens.get(1) == Some(&"base") {
                let anchor_indices = tokens.iter().enumerate().filter_map(|(index, token)| (*token == "<anchor").then_some(index)).collect::<Vec<_>>();
                let Some(&first_anchor) = anchor_indices.first() else {
                    continue;
                };
                let glyph_names = clean_feature_class(&tokens[2..first_anchor]);
                for (anchor_number, &anchor_index) in anchor_indices.iter().enumerate() {
                    let Some((x, y)) = parse_feature_anchor(&tokens, anchor_index) else {
                        continue;
                    };
                    let end = anchor_indices.get(anchor_number + 1).copied().unwrap_or(tokens.len());
                    let Some(class_name) = tokens.get(anchor_index + 3..end).and_then(|tokens| tokens.iter().find(|token| token.starts_with('@'))).map(|token| token.trim_start_matches('@')).filter(|name| !name.is_empty()) else {
                        continue;
                    };
                    for glyph_name in &glyph_names {
                        let Some(&glyph_id) = glyph_ids.get(glyph_name.as_str()) else {
                            continue;
                        };
                        base_names.entry(GlyphId16::new(glyph_id)).or_default().push((class_name.to_string(), gpos::AnchorTable::format_1(x, y)));
                    }
                }
            } else if tokens.first() == Some(&"pos") && tokens.get(1) == Some(&"mark") {
                source_mark_to_mark = true;
            }
        }
    }
    let mark_names_for_mark = mark_names.clone();
    let base_names_for_mark = base_names.clone();
    if !mark_names.is_empty() && !base_names.is_empty() {
        let mut classes = BTreeMap::<String, u16>::new();
        for anchors in base_names.values() {
            for (name, _) in anchors {
                let next = classes.len() as u16;
                classes.entry(name.clone()).or_insert(next);
            }
        }
        let mark_coverage: layout::CoverageTable = mark_names.keys().copied().collect();
        let base_coverage: layout::CoverageTable = base_names.keys().copied().collect();
        let mark_array = gpos::MarkArray::new(
            mark_names
                .into_values()
                .map(|anchors| {
                    let (name, anchor) = anchors.into_iter().next().unwrap();
                    gpos::MarkRecord::new(*classes.get(&name).unwrap_or(&0), anchor)
                })
                .collect(),
        );
        let base_array = gpos::BaseArray::new(
            base_names
                .into_values()
                .map(|anchors| {
                    let mut class_anchors = vec![None; classes.len()];
                    for (name, anchor) in anchors {
                        if let Some(&class) = classes.get(&name) {
                            class_anchors[class as usize] = Some(anchor);
                        }
                    }
                    gpos::BaseRecord::new(class_anchors)
                })
                .collect(),
        );
        let mark_base = gpos::MarkBasePosFormat1::new(mark_coverage, base_coverage, mark_array, base_array);
        let lookup = layout::Lookup::new(layout::LookupFlag::empty(), vec![mark_base]);
        feature_indices.push((Tag::new(b"mark"), lookups.len() as u16));
        lookups.push(gpos::PositionLookup::MarkToBase(lookup));
    }
    let mut ligature_names = BTreeMap::<GlyphId16, Vec<(usize, String, gpos::AnchorTable)>>::new();
    for name in project.glyphs.keys() {
        let Some(&glyph_id) = glyph_ids.get(name.as_str()) else {
            continue;
        };
        for anchor in project.anchors_for_glyph(name) {
            let Some((anchor_name, suffix)) = anchor.name.rsplit_once('_') else {
                continue;
            };
            let Ok(component) = suffix.parse::<usize>() else {
                continue;
            };
            if component == 0 || anchor_name.is_empty() || anchor_name.starts_with('_') {
                continue;
            }
            let (Ok(x), Ok(y)) = (checked_i16(anchor.x, "合字アンカーX"), checked_i16(anchor.y, "合字アンカーY")) else {
                continue;
            };
            ligature_names.entry(GlyphId16::new(glyph_id)).or_default().push((component, anchor_name.to_string(), gpos::AnchorTable::format_1(x, y)));
        }
    }
    for (_feature_tag, block) in &raw_feature_blocks {
        for statement in block.split(';') {
            let tokens = statement.split_whitespace().collect::<Vec<_>>();
            if tokens.first() != Some(&"pos") || tokens.get(1) != Some(&"ligature") {
                continue;
            }
            let operand_indices = tokens.iter().enumerate().filter_map(|(index, token)| (*token == "<anchor" || *token == "NULL").then_some(index)).collect::<Vec<_>>();
            let Some(&first_operand) = operand_indices.first() else {
                continue;
            };
            let glyph_names = clean_feature_class(&tokens[2..first_operand]);
            for (component_index, &operand_index) in operand_indices.iter().enumerate() {
                if tokens.get(operand_index) == Some(&"NULL") {
                    continue;
                }
                let anchor_index = operand_index;
                let Some((x, y)) = parse_feature_anchor(&tokens, anchor_index) else {
                    continue;
                };
                let end = operand_indices.get(component_index + 1).copied().unwrap_or(tokens.len());
                let Some(class_name) = tokens.get(anchor_index + 3..end).and_then(|tokens| tokens.iter().find(|token| token.starts_with('@'))).map(|token| token.trim_start_matches('@')).filter(|name| !name.is_empty()) else {
                    continue;
                };
                for glyph_name in &glyph_names {
                    let Some(&glyph_id) = glyph_ids.get(glyph_name.as_str()) else {
                        continue;
                    };
                    ligature_names.entry(GlyphId16::new(glyph_id)).or_default().push((component_index + 1, class_name.to_string(), gpos::AnchorTable::format_1(x, y)));
                }
            }
        }
    }
    if !mark_names_for_mark.is_empty() && !ligature_names.is_empty() {
        let mut classes = BTreeMap::<String, u16>::new();
        for anchors in ligature_names.values() {
            for (_, name, _) in anchors {
                let next = classes.len() as u16;
                classes.entry(name.clone()).or_insert(next);
            }
        }
        let mark_coverage: layout::CoverageTable = mark_names_for_mark.keys().copied().collect();
        let ligature_coverage: layout::CoverageTable = ligature_names.keys().copied().collect();
        let mark_array = gpos::MarkArray::new(
            mark_names_for_mark
                .values()
                .map(|anchors| {
                    let (name, anchor) = anchors.first().unwrap();
                    gpos::MarkRecord::new(*classes.get(name).unwrap_or(&0), anchor.clone())
                })
                .collect(),
        );
        let ligature_array = gpos::LigatureArray::new(
            ligature_names
                .values()
                .map(|anchors| {
                    let component_count = anchors.iter().map(|(component, _, _)| *component).max().unwrap_or(0);
                    let component_records = (1..=component_count)
                        .map(|component| {
                            let mut class_anchors = vec![None; classes.len()];
                            for (anchor_component, name, anchor) in anchors {
                                if *anchor_component == component {
                                    if let Some(&class) = classes.get(name) {
                                        class_anchors[class as usize] = Some(anchor.clone());
                                    }
                                }
                            }
                            gpos::ComponentRecord::new(class_anchors)
                        })
                        .collect();
                    gpos::LigatureAttach::new(component_records)
                })
                .collect(),
        );
        let mark_ligature = gpos::MarkLigPosFormat1::new(mark_coverage, ligature_coverage, mark_array, ligature_array);
        let lookup = layout::Lookup::new(layout::LookupFlag::empty(), vec![mark_ligature]);
        feature_indices.push((Tag::new(b"mark"), lookups.len() as u16));
        lookups.push(gpos::PositionLookup::MarkToLig(lookup));
    }
    if !mark_names_for_mark.is_empty() && !base_names_for_mark.is_empty() {
        let mut classes = BTreeMap::<String, u16>::new();
        for anchors in base_names_for_mark.values() {
            for (name, _) in anchors {
                let next = classes.len() as u16;
                classes.entry(name.clone()).or_insert(next);
            }
        }
        let mark1: Vec<_> = mark_names_for_mark
            .iter()
            .filter_map(|(glyph_id, anchors)| {
                let (name, anchor) = anchors.first()?.clone();
                Some((*glyph_id, gpos::MarkRecord::new(*classes.get(&name).unwrap_or(&0), anchor)))
            })
            .collect();
        let mark2: Vec<_> = base_names_for_mark
            .iter()
            .filter(|(glyph_id, _)| mark_names_for_mark.contains_key(glyph_id))
            .map(|(_, anchors)| {
                let mut class_anchors = vec![None; classes.len()];
                for (name, anchor) in anchors {
                    if let Some(&class) = classes.get(name) {
                        class_anchors[class as usize] = Some(anchor.clone());
                    }
                }
                gpos::Mark2Record::new(class_anchors)
            })
            .collect();
        if !mark1.is_empty() && !mark2.is_empty() {
            let mark1_coverage: layout::CoverageTable = mark1.iter().map(|(id, _)| *id).collect();
            let mark2_coverage: layout::CoverageTable = base_names_for_mark.keys().filter(|id| mark_names_for_mark.contains_key(id)).copied().collect();
            let mark1_array = gpos::MarkArray::new(mark1.into_iter().map(|(_, record)| record).collect());
            let mark2_array = gpos::Mark2Array::new(mark2);
            let mark_mark = gpos::MarkMarkPosFormat1::new(mark1_coverage, mark2_coverage, mark1_array, mark2_array);
            let lookup = layout::Lookup::new(layout::LookupFlag::empty(), vec![mark_mark]);
            feature_indices.push((Tag::new(b"mkmk"), lookups.len() as u16));
            lookups.push(gpos::PositionLookup::MarkToMark(lookup));
        }
    }
    if source_mark_to_mark && !mark_names_for_mark.is_empty() {
        let mut classes = BTreeMap::<String, u16>::new();
        for anchors in mark_names_for_mark.values() {
            for (name, _) in anchors {
                let next = classes.len() as u16;
                classes.entry(name.clone()).or_insert(next);
            }
        }
        let mark1 = mark_names_for_mark
            .iter()
            .filter_map(|(glyph_id, anchors)| {
                let (name, anchor) = anchors.first()?.clone();
                Some((*glyph_id, gpos::MarkRecord::new(*classes.get(&name).unwrap_or(&0), anchor)))
            })
            .collect::<Vec<_>>();
        let mark2 = mark_names_for_mark
            .values()
            .map(|anchors| {
                let mut class_anchors = vec![None; classes.len()];
                for (name, anchor) in anchors {
                    if let Some(&class) = classes.get(name) {
                        class_anchors[class as usize] = Some(anchor.clone());
                    }
                }
                gpos::Mark2Record::new(class_anchors)
            })
            .collect::<Vec<_>>();
        if !mark1.is_empty() && !mark2.is_empty() {
            let mark1_coverage: layout::CoverageTable = mark1.iter().map(|(id, _)| *id).collect();
            let mark2_coverage: layout::CoverageTable = mark_names_for_mark.keys().copied().collect();
            let mark1_array = gpos::MarkArray::new(mark1.into_iter().map(|(_, record)| record).collect());
            let mark2_array = gpos::Mark2Array::new(mark2);
            let mark_mark = gpos::MarkMarkPosFormat1::new(mark1_coverage, mark2_coverage, mark1_array, mark2_array);
            let lookup = layout::Lookup::new(layout::LookupFlag::empty(), vec![mark_mark]);
            feature_indices.push((Tag::new(b"mkmk"), lookups.len() as u16));
            lookups.push(gpos::PositionLookup::MarkToMark(lookup));
        }
    }
    let mut kerning_variations = Vec::<(layout::ConditionSet, u16)>::new();
    if project.masters.len() >= 2 && project.kerning_by_master.len() >= 2 {
        let axis_values = variable_master_axis_values(project);
        let default_master_id = project.default_master_id.as_str();
        for master in &project.masters {
            if master.id == default_master_id {
                continue;
            }
            let Some(kerning) = project.kerning_by_master.get(&master.id) else {
                continue;
            };
            if kerning == &project.kerning {
                continue;
            }
            let Some(lookup) = build_direct_kerning_lookup(kerning, glyph_ids) else {
                continue;
            };
            let conditions = axis_values
                .iter()
                .enumerate()
                .filter_map(|(axis_index, (_, values))| {
                    let value = values.get(&master.id).copied()?;
                    let mut sorted = values.values().copied().collect::<Vec<_>>();
                    sorted.sort_by(f64::total_cmp);
                    sorted.dedup_by(|left, right| (*left - *right).abs() < f64::EPSILON);
                    if sorted.len() < 2 {
                        return None;
                    }
                    let index = sorted.iter().position(|candidate| (*candidate - value).abs() < f64::EPSILON)?;
                    let min = if index == 0 { -1.0 } else { (sorted[index - 1] + value) / 2.0 };
                    let max = if index + 1 == sorted.len() { 1.0 } else { (value + sorted[index + 1]) / 2.0 };
                    let normalized = |coordinate: f64| {
                        let (axis_min, default, axis_max) = project_axis_bounds(project, axis_index);
                        if coordinate <= default {
                            ((coordinate - default) / (default - axis_min).max(f64::EPSILON)).clamp(-1.0, 1.0)
                        } else {
                            ((coordinate - default) / (axis_max - default).max(f64::EPSILON)).clamp(-1.0, 1.0)
                        }
                    };
                    Some(layout::Condition::format_1_axis_range(axis_index as u16, font_types::F2Dot14::from_f32(normalized(min) as f32), font_types::F2Dot14::from_f32(normalized(max) as f32)))
                })
                .collect::<Vec<_>>();
            if conditions.is_empty() {
                continue;
            }
            let lookup_index = lookups.len() as u16;
            lookups.push(lookup);
            kerning_variations.push((layout::ConditionSet::new(conditions), lookup_index));
        }
    }
    if lookups.is_empty() {
        return None;
    }
    let feature_references = parse_feature_references(&source);
    loop {
        let mut changed = false;
        for (parent, child) in &feature_references {
            let child_indices = feature_indices.iter().filter(|(tag, _)| tag == child).map(|(_, index)| *index).collect::<Vec<_>>();
            for index in child_indices {
                if !feature_indices.iter().any(|(tag, existing)| tag == parent && *existing == index) {
                    feature_indices.push((*parent, index));
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let feature_index_by_tag = feature_indices.iter().fold(BTreeMap::<Tag, Vec<u16>>::new(), |mut map, (tag, index)| {
        map.entry(*tag).or_default().push(*index);
        map
    });
    let feature_tags = feature_index_by_tag.keys().copied().collect::<Vec<_>>();
    let features = layout::FeatureList::new(feature_index_by_tag.iter().map(|(tag, indices)| layout::FeatureRecord::new(*tag, layout::Feature::new(feature_params_for_tag(*tag, &source, unicode_by_glyph), indices.clone()))).collect());
    let lookups = if feature_uses_extension_lookups(&source) { lookups.into_iter().map(wrap_gpos_extension_lookup).collect() } else { lookups };
    let lookups = layout::LookupList::new(lookups);
    let scripts = build_script_list(&source, &feature_tags);
    let mut table = gpos::Gpos::new(scripts, features, lookups);
    if let Some(kern_feature_index) = feature_tags.iter().position(|tag| *tag == Tag::new(b"kern")) {
        let records = kerning_variations
            .into_iter()
            .map(|(condition_set, lookup_index)| layout::FeatureVariationRecord::new(Some(condition_set), Some(layout::FeatureTableSubstitution::new(vec![layout::FeatureTableSubstitutionRecord::new(kern_feature_index as u16, layout::Feature::new(None, vec![lookup_index]))]))))
            .collect::<Vec<_>>();
        if !records.is_empty() {
            table.feature_variations = layout::FeatureVariations::new(records).into();
        }
    }
    write_fonts::dump_table(&table).ok()
}
