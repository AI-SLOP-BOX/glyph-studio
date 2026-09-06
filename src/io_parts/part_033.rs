
/// Restores simple GPOS SinglePos lookups as editable four-value `pos` rules.
/// Device tables and more complex positioning remain preserved byte-for-byte.
fn import_simple_gpos_features(face: &ttf_parser::Face<'_>, names: &[String]) -> String {
    let Some(gpos) = face.tables().gpos else {
        return String::new();
    };
    let mut features = std::collections::BTreeMap::<String, Vec<String>>::new();
    let mut class_definitions = Vec::new();
    let mut class_serial = 0_usize;
    for feature in gpos.features {
        let tag = feature.tag.to_string();
        if tag.len() != 4 || !tag.is_ascii() || tag.eq_ignore_ascii_case("kern") {
            continue;
        }
        let rules = features.entry(tag).or_default();
        if let Some(first_lookup_index) = feature.lookup_indices.get(0) {
            if let Some(lookup) = gpos.lookups.get(first_lookup_index) {
                if let Some(flags) = imported_lookup_flag_source!(lookup) {
                    rules.push(flags);
                }
            }
        }
        for lookup_index in feature.lookup_indices {
            let Some(lookup) = gpos.lookups.get(lookup_index) else {
                continue;
            };
            for subtable in lookup
                .subtables
                .into_iter::<ttf_parser::gpos::PositioningSubtable>()
            {
                match subtable {
                    ttf_parser::gpos::PositioningSubtable::Single(
                        ttf_parser::gpos::SingleAdjustment::Format1 { coverage, value },
                    ) => {
                        for raw_id in 0..names.len() {
                            let source_id = ttf_parser::GlyphId(raw_id as u16);
                            if coverage.get(source_id).is_none() {
                                continue;
                            }
                            let (Some(source), Some(value)) = (
                                feature_glyph_name(names, source_id),
                                format_gpos_value(value),
                            ) else {
                                continue;
                            };
                            rules.push(format!("pos {source} {value};"));
                        }
                    }
                    ttf_parser::gpos::PositioningSubtable::Single(
                        ttf_parser::gpos::SingleAdjustment::Format2 { coverage, values },
                    ) => {
                        for raw_id in 0..names.len() {
                            let source_id = ttf_parser::GlyphId(raw_id as u16);
                            let Some(coverage_index) = coverage.get(source_id) else {
                                continue;
                            };
                            let (Some(source), Some(value)) = (
                                feature_glyph_name(names, source_id),
                                values.get(coverage_index).and_then(format_gpos_value),
                            ) else {
                                continue;
                            };
                            rules.push(format!("pos {source} {value};"));
                        }
                    }
                    ttf_parser::gpos::PositioningSubtable::Pair(
                        ttf_parser::gpos::PairAdjustment::Format1 { coverage, sets },
                    ) => {
                        for raw_id in 0..names.len() {
                            let first_id = ttf_parser::GlyphId(raw_id as u16);
                            let Some(coverage_index) = coverage.get(first_id) else {
                                continue;
                            };
                            let Some(pair_set) = sets.get(coverage_index) else {
                                continue;
                            };
                            let Some(first_name) = feature_glyph_name(names, first_id) else {
                                continue;
                            };
                            for second_raw_id in 0..names.len() {
                                let second_id = ttf_parser::GlyphId(second_raw_id as u16);
                                let Some((first_value, second_value)) = pair_set.get(second_id)
                                else {
                                    continue;
                                };
                                let (Some(second_name), Some(value)) = (
                                    feature_glyph_name(names, second_id),
                                    format_gpos_pair(first_value, second_value),
                                ) else {
                                    continue;
                                };
                                rules.push(format!("pos {first_name} {second_name} {value};"));
                            }
                        }
                    }
                    ttf_parser::gpos::PositioningSubtable::Pair(
                        ttf_parser::gpos::PairAdjustment::Format2 {
                            coverage,
                            classes: (left_classes, right_classes),
                            matrix,
                        },
                    ) => {
                        let mut left_members = vec![Vec::<String>::new(); names.len()];
                        let mut right_members = vec![Vec::<String>::new(); names.len()];
                        let mut max_left_class = 0_u16;
                        let mut max_right_class = 0_u16;
                        for raw_id in 0..names.len() {
                            let glyph_id = ttf_parser::GlyphId(raw_id as u16);
                            let Some(name) = feature_glyph_name(names, glyph_id) else {
                                continue;
                            };
                            let left_class = left_classes.get(glyph_id);
                            let right_class = right_classes.get(glyph_id);
                            max_left_class = max_left_class.max(left_class);
                            max_right_class = max_right_class.max(right_class);
                            if coverage.get(glyph_id).is_some() {
                                left_members[usize::from(left_class)].push(name.to_string());
                            }
                            right_members[usize::from(right_class)].push(name.to_string());
                        }
                        let mut left_names = Vec::new();
                        let mut right_names = Vec::new();
                        class_serial += 1;
                        for class in 0..=max_left_class {
                            let members = &left_members[usize::from(class)];
                            if members.is_empty() {
                                left_names.push(None);
                            } else {
                                let class_name = format!("@GS{class_serial}L{class}");
                                class_definitions
                                    .push(format!("{class_name} = [{}];", members.join(" ")));
                                left_names.push(Some(class_name));
                            }
                        }
                        for class in 0..=max_right_class {
                            let members = &right_members[usize::from(class)];
                            if members.is_empty() {
                                right_names.push(None);
                            } else {
                                let class_name = format!("@GS{class_serial}R{class}");
                                class_definitions
                                    .push(format!("{class_name} = [{}];", members.join(" ")));
                                right_names.push(Some(class_name));
                            }
                        }
                        for left_class in 0..=max_left_class {
                            for right_class in 0..=max_right_class {
                                let Some((first, second)) = matrix.get((left_class, right_class))
                                else {
                                    continue;
                                };
                                let Some(value) = format_gpos_pair(first, second) else {
                                    continue;
                                };
                                let (Some(left_name), Some(right_name)) = (
                                    left_names[usize::from(left_class)].as_deref(),
                                    right_names[usize::from(right_class)].as_deref(),
                                ) else {
                                    continue;
                                };
                                rules.push(format!("pos {left_name} {right_name} {value};"));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let feature_source = features
        .into_iter()
        .filter_map(|(tag, rules)| {
            (!rules.is_empty()).then(|| format!("feature {tag} {{ {} }} {tag};", rules.join(" ")))
        })
        .collect::<Vec<_>>()
        .join(" ");
    class_definitions
        .into_iter()
        .chain((!feature_source.is_empty()).then_some(feature_source))
        .collect::<Vec<_>>()
        .join(" ")
}
