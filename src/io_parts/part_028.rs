
fn imported_contextual_gpos_features(font: &FontRef<'_>, names: &[String]) -> String {
    let Ok(gpos) = font.gpos() else {
        return String::new();
    };
    let (Ok(features), Ok(lookups)) = (gpos.feature_list(), gpos.lookup_list()) else {
        return String::new();
    };
    let mut feature_rules = std::collections::BTreeMap::<String, Vec<String>>::new();
    let mut class_definitions = Vec::new();
    let mut class_serial = 0_usize;
    let coverage_source = |coverage: &read_fonts::tables::layout::CoverageTable<'_>,
                           prefix: &str,
                           definitions: &mut Vec<String>,
                           serial: &mut usize|
     -> Option<String> {
        let glyphs = coverage
            .iter()
            .filter_map(|glyph| names.get(usize::from(glyph.to_u16())))
            .cloned()
            .collect::<Vec<_>>();
        if glyphs.is_empty() {
            return None;
        }
        *serial += 1;
        let class_name = format!("@{}{}", prefix, *serial);
        definitions.push(format!("{class_name} = [{}];", glyphs.join(" ")));
        Some(class_name)
    };
    let class_source = |glyphs: Vec<String>,
                        prefix: &str,
                        definitions: &mut Vec<String>,
                        serial: &mut usize|
     -> Option<String> {
        if glyphs.is_empty() {
            return None;
        }
        *serial += 1;
        let class_name = format!("@{}{}", prefix, *serial);
        definitions.push(format!("{class_name} = [{}];", glyphs.join(" ")));
        Some(class_name)
    };
    let value_source =
        |value: [i16; 4]| format!("<{} {} {} {}>", value[0], value[1], value[2], value[3]);
    for record in features.feature_records() {
        let tag = record.feature_tag().to_string();
        if tag.len() != 4 || !tag.is_ascii() || tag == "kern" {
            continue;
        }
        let Ok(feature) = record.feature(features.offset_data()) else {
            continue;
        };
        let rules = feature_rules.entry(tag).or_default();
        for lookup_index in feature.lookup_list_indices() {
            let Ok(lookup) = lookups.lookups().get(usize::from(lookup_index.get())) else {
                continue;
            };
            if let Some(flags) = imported_read_lookup_flag_source(lookup.lookup_flag()) {
                rules.push(flags);
            }
            let Ok(PositionSubtables::Contextual(subtables)) = lookup.subtables() else {
                continue;
            };
            for table in subtables.iter().flatten() {
                let (records, mut tokens, _target_coverage, class_based) = match table {
                    read_fonts::tables::layout::SequenceContext::Format2(context) => {
                        let (Ok(coverage), Ok(class_def)) =
                            (context.coverage(), context.class_def())
                        else {
                            continue;
                        };
                        let mut found = None;
                        for (first_glyph, rule_set) in coverage
                            .iter()
                            .zip(context.class_seq_rule_sets().iter().flatten())
                        {
                            let Ok(rule_set) = rule_set else {
                                continue;
                            };
                            let first_class = class_def.get(first_glyph);
                            for rule in rule_set.class_seq_rules().iter() {
                                let Ok(rule) = rule else {
                                    continue;
                                };
                                if rule.seq_lookup_records().len() == 1 {
                                    found = Some((first_class, rule, class_def.clone()));
                                    break;
                                }
                            }
                            if found.is_some() {
                                break;
                            }
                        }
                        let Some((first_class, rule, class_def)) = found else {
                            continue;
                        };
                        let records = rule.seq_lookup_records();
                        let mut tokens = Vec::new();
                        let first_glyphs = coverage
                            .iter()
                            .filter(|glyph| class_def.get(*glyph) == first_class)
                            .filter_map(|glyph| names.get(usize::from(glyph.to_u16())))
                            .cloned()
                            .collect::<Vec<_>>();
                        let Some(first) = class_source(
                            first_glyphs,
                            "GPClass",
                            &mut class_definitions,
                            &mut class_serial,
                        ) else {
                            continue;
                        };
                        tokens.push(first);
                        for class in rule.input_sequence() {
                            let class = class.get();
                            let glyphs = (0..names.len())
                                .filter_map(|raw_id| {
                                    let glyph = GlyphId::new(raw_id as u32);
                                    (class_def.get(glyph) == class)
                                        .then(|| names.get(raw_id).cloned())
                                        .flatten()
                                })
                                .collect::<Vec<_>>();
                            let Some(token) = class_source(
                                glyphs,
                                "GPClass",
                                &mut class_definitions,
                                &mut class_serial,
                            ) else {
                                tokens.clear();
                                break;
                            };
                            tokens.push(token);
                        }
                        (records, tokens, coverage, true)
                    }
                    read_fonts::tables::layout::SequenceContext::Format3(context) => {
                        let records = context.seq_lookup_records();
                        if records.len() != 1 {
                            continue;
                        }
                        let coverages = context.coverages().iter().flatten().collect::<Vec<_>>();
                        let mut tokens = Vec::new();
                        for coverage in &coverages {
                            let Some(class) = coverage_source(
                                coverage,
                                "GPContext",
                                &mut class_definitions,
                                &mut class_serial,
                            ) else {
                                tokens.clear();
                                break;
                            };
                            tokens.push(class);
                        }
                        let target_index = usize::from(records[0].sequence_index());
                        if target_index >= coverages.len() {
                            continue;
                        }
                        (records, tokens, coverages[target_index].clone(), false)
                    }
                    read_fonts::tables::layout::SequenceContext::Format1(context) => {
                        let Ok(coverage) = context.coverage() else {
                            continue;
                        };
                        let mut found = None;
                        for rule_set in context.seq_rule_sets().iter().flatten() {
                            let Ok(rule_set) = rule_set else {
                                continue;
                            };
                            for rule in rule_set.seq_rules().iter() {
                                let Ok(rule) = rule else {
                                    continue;
                                };
                                if rule.seq_lookup_records().len() == 1 {
                                    found = Some((rule, coverage.clone()));
                                    break;
                                }
                            }
                            if found.is_some() {
                                break;
                            }
                        }
                        let Some((rule, target_coverage)) = found else {
                            continue;
                        };
                        let records = rule.seq_lookup_records();
                        let mut tokens = Vec::new();
                        let Some(first) = coverage_source(
                            &target_coverage,
                            "GPContext",
                            &mut class_definitions,
                            &mut class_serial,
                        ) else {
                            continue;
                        };
                        tokens.push(first);
                        for glyph in rule.input_sequence() {
                            let Some(name) = names.get(usize::from(glyph.get().to_u16())) else {
                                tokens.clear();
                                break;
                            };
                            tokens.push((*name).clone());
                        }
                        (records, tokens, target_coverage, false)
                    }
                };
                if tokens.is_empty() {
                    continue;
                }
                let lookup_record = records[0];
                let Ok(target_lookup) = lookups
                    .lookups()
                    .get(usize::from(lookup_record.lookup_list_index()))
                else {
                    continue;
                };
                let positions = imported_single_positioning_map(&target_lookup);
                let target_index = usize::from(lookup_record.sequence_index());
                if positions.is_empty() || target_index >= tokens.len() {
                    continue;
                }
                for (source, value) in positions {
                    let Some(name) = names.get(usize::from(source)) else {
                        continue;
                    };
                    if class_based {
                        tokens[target_index] =
                            format!("{}'", tokens[target_index].trim_end_matches('\''));
                    } else {
                        tokens[target_index] = format!("{name}'");
                    }
                    rules.push(format!("pos {} {};", tokens.join(" "), value_source(value)));
                }
            }
        }
        for lookup_index in feature.lookup_list_indices() {
            let Ok(lookup) = lookups.lookups().get(usize::from(lookup_index.get())) else {
                continue;
            };
            if let Some(flags) = imported_read_lookup_flag_source(lookup.lookup_flag()) {
                rules.push(flags);
            }
            let Ok(PositionSubtables::ChainContextual(subtables)) = lookup.subtables() else {
                continue;
            };
            for table in subtables.iter().flatten() {
                if let read_fonts::tables::layout::ChainedSequenceContext::Format1(context) = table
                {
                    let Ok(coverage) = context.coverage() else {
                        continue;
                    };
                    let mut found = None;
                    for (first_glyph, rule_set) in coverage
                        .iter()
                        .zip(context.chained_seq_rule_sets().iter().flatten())
                    {
                        let Ok(rule_set) = rule_set else {
                            continue;
                        };
                        for rule in rule_set.chained_seq_rules().iter() {
                            let Ok(rule) = rule else {
                                continue;
                            };
                            if rule.seq_lookup_records().len() == 1 {
                                found = Some((first_glyph, rule));
                                break;
                            }
                        }
                        if found.is_some() {
                            break;
                        }
                    }
                    let Some((first_glyph, rule)) = found else {
                        continue;
                    };
                    let records = rule.seq_lookup_records();
                    let target_index = usize::from(records[0].sequence_index());
                    let mut tokens = Vec::new();
                    for glyph in rule.backtrack_sequence() {
                        let Some(name) = names.get(usize::from(glyph.get().to_u16())) else {
                            tokens.clear();
                            break;
                        };
                        tokens.push((*name).clone());
                    }
                    let Some(first_name) = names.get(usize::from(first_glyph.to_u16())) else {
                        continue;
                    };
                    tokens.push((*first_name).clone());
                    for glyph in rule.input_sequence() {
                        let Some(name) = names.get(usize::from(glyph.get().to_u16())) else {
                            tokens.clear();
                            break;
                        };
                        tokens.push((*name).clone());
                    }
                    for glyph in rule.lookahead_sequence() {
                        let Some(name) = names.get(usize::from(glyph.get().to_u16())) else {
                            tokens.clear();
                            break;
                        };
                        tokens.push((*name).clone());
                    }
                    if tokens.is_empty() {
                        continue;
                    }
                    let Ok(target_lookup) = lookups
                        .lookups()
                        .get(usize::from(records[0].lookup_list_index()))
                    else {
                        continue;
                    };
                    let positions = imported_single_positioning_map(&target_lookup);
                    let token_index = rule.backtrack_sequence().len() + target_index;
                    for (source, value) in positions {
                        let Some(name) = names.get(usize::from(source)) else {
                            continue;
                        };
                        if token_index >= tokens.len() {
                            continue;
                        }
                        tokens[token_index] = format!("{name}'");
                        rules.push(format!("pos {} {};", tokens.join(" "), value_source(value)));
                    }
                    continue;
                }
                if let read_fonts::tables::layout::ChainedSequenceContext::Format2(context) = table
                {
                    let (Ok(coverage), Ok(backtrack_def), Ok(input_def), Ok(lookahead_def)) = (
                        context.coverage(),
                        context.backtrack_class_def(),
                        context.input_class_def(),
                        context.lookahead_class_def(),
                    ) else {
                        continue;
                    };
                    let mut found = None;
                    for (first_glyph, rule_set) in coverage
                        .iter()
                        .zip(context.chained_class_seq_rule_sets().iter().flatten())
                    {
                        let Ok(rule_set) = rule_set else {
                            continue;
                        };
                        for rule in rule_set.chained_class_seq_rules().iter() {
                            let Ok(rule) = rule else {
                                continue;
                            };
                            if rule.seq_lookup_records().len() == 1 {
                                found = Some((first_glyph, rule));
                                break;
                            }
                        }
                        if found.is_some() {
                            break;
                        }
                    }
                    let Some((first_glyph, rule)) = found else {
                        continue;
                    };
                    let records = rule.seq_lookup_records();
                    let class_glyphs = |class_def: &read_fonts::tables::layout::ClassDef<'_>,
                                        class: u16|
                     -> Vec<String> {
                        (0..names.len())
                            .filter_map(|raw_id| {
                                let glyph = GlyphId::new(raw_id as u32);
                                (class_def.get(glyph) == class)
                                    .then(|| names.get(raw_id).cloned())
                                    .flatten()
                            })
                            .collect()
                    };
                    let class_token = |class_def: &read_fonts::tables::layout::ClassDef<'_>,
                                       class: u16,
                                       definitions: &mut Vec<String>,
                                       serial: &mut usize|
                     -> Option<String> {
                        class_source(
                            class_glyphs(class_def, class),
                            "GPChainClass",
                            definitions,
                            serial,
                        )
                    };
                    let mut tokens = Vec::new();
                    for class in rule.backtrack_sequence() {
                        let Some(token) = class_token(
                            &backtrack_def,
                            class.get(),
                            &mut class_definitions,
                            &mut class_serial,
                        ) else {
                            tokens.clear();
                            break;
                        };
                        tokens.push(token);
                    }
                    let first_class = input_def.get(first_glyph);
                    let Some(token) = class_token(
                        &input_def,
                        first_class,
                        &mut class_definitions,
                        &mut class_serial,
                    ) else {
                        continue;
                    };
                    tokens.push(token);
                    for class in rule.input_sequence() {
                        let Some(token) = class_token(
                            &input_def,
                            class.get(),
                            &mut class_definitions,
                            &mut class_serial,
                        ) else {
                            tokens.clear();
                            break;
                        };
                        tokens.push(token);
                    }
                    for class in rule.lookahead_sequence() {
                        let Some(token) = class_token(
                            &lookahead_def,
                            class.get(),
                            &mut class_definitions,
                            &mut class_serial,
                        ) else {
                            tokens.clear();
                            break;
                        };
                        tokens.push(token);
                    }
                    if tokens.is_empty() {
                        continue;
                    }
                    let Ok(target_lookup) = lookups
                        .lookups()
                        .get(usize::from(records[0].lookup_list_index()))
                    else {
                        continue;
                    };
                    let positions = imported_single_positioning_map(&target_lookup);
                    let target_index =
                        rule.backtrack_sequence().len() + usize::from(records[0].sequence_index());
                    if target_index >= tokens.len() {
                        continue;
                    }
                    tokens[target_index] = format!("{}'", tokens[target_index]);
                    let target_class = if records[0].sequence_index() == 0 {
                        first_class
                    } else {
                        let Some(class) = rule
                            .input_sequence()
                            .get(usize::from(records[0].sequence_index()) - 1)
                        else {
                            continue;
                        };
                        class.get()
                    };
                    for (source, value) in positions {
                        if input_def.get(GlyphId::new(u32::from(source))) != target_class {
                            continue;
                        }
                        rules.push(format!("pos {} {};", tokens.join(" "), value_source(value)));
                    }
                    continue;
                }
                let read_fonts::tables::layout::ChainedSequenceContext::Format3(context) = table
                else {
                    continue;
                };
                let records = context.seq_lookup_records();
                if records.len() != 1 {
                    continue;
                }
                let input_coverages = context
                    .input_coverages()
                    .iter()
                    .flatten()
                    .collect::<Vec<_>>();
                let target_index = usize::from(records[0].sequence_index());
                if input_coverages.is_empty() || target_index >= input_coverages.len() {
                    continue;
                }
                let mut tokens = Vec::new();
                for coverage in context.backtrack_coverages().iter().flatten() {
                    let Some(class) = coverage_source(
                        &coverage,
                        "GPChainB",
                        &mut class_definitions,
                        &mut class_serial,
                    ) else {
                        tokens.clear();
                        break;
                    };
                    tokens.push(class);
                }
                if tokens.is_empty() && context.backtrack_glyph_count() != 0 {
                    continue;
                }
                for coverage in &input_coverages {
                    let Some(class) = coverage_source(
                        coverage,
                        "GPChainI",
                        &mut class_definitions,
                        &mut class_serial,
                    ) else {
                        tokens.clear();
                        break;
                    };
                    tokens.push(class);
                }
                for coverage in context.lookahead_coverages().iter().flatten() {
                    let Some(class) = coverage_source(
                        &coverage,
                        "GPChainL",
                        &mut class_definitions,
                        &mut class_serial,
                    ) else {
                        tokens.clear();
                        break;
                    };
                    tokens.push(class);
                }
                if tokens.is_empty() {
                    continue;
                }
                let Ok(target_lookup) = lookups
                    .lookups()
                    .get(usize::from(records[0].lookup_list_index()))
                else {
                    continue;
                };
                let positions = imported_single_positioning_map(&target_lookup);
                let token_index = usize::from(context.backtrack_glyph_count()) + target_index;
                for (source, value) in positions {
                    if input_coverages[target_index]
                        .get(GlyphId::new(u32::from(source)))
                        .is_none()
                    {
                        continue;
                    }
                    let Some(name) = names.get(usize::from(source)) else {
                        continue;
                    };
                    if token_index >= tokens.len() {
                        continue;
                    }
                    tokens[token_index] = format!("{name}'");
                    rules.push(format!("pos {} {};", tokens.join(" "), value_source(value)));
                }
            }
        }
    }
    let features = feature_rules
        .into_iter()
        .filter_map(|(tag, rules)| {
            (!rules.is_empty()).then(|| format!("feature {tag} {{ {} }} {tag};", rules.join(" ")))
        })
        .collect::<Vec<_>>();
    class_definitions
        .into_iter()
        .chain(features)
        .collect::<Vec<_>>()
        .join(" ")
}
