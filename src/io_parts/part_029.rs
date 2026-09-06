
fn imported_contextual_gsub_features(font: &FontRef<'_>, names: &[String]) -> String {
    let Ok(gsub) = font.gsub() else {
        return String::new();
    };
    let (Ok(features), Ok(lookups)) = (gsub.feature_list(), gsub.lookup_list()) else {
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
    for record in features.feature_records() {
        let tag = record.feature_tag().to_string();
        if tag.len() != 4 || !tag.is_ascii() {
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
            let Ok(subtables) = lookup.subtables() else {
                continue;
            };
            match subtables {
                SubstitutionSubtables::Contextual(subtables) => {
                    for table in subtables.iter().flatten() {
                        if let read_fonts::tables::layout::SequenceContext::Format2(context) = table
                        {
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
                            let Some(first) = class_source(
                                coverage
                                    .iter()
                                    .filter(|glyph| class_def.get(*glyph) == first_class)
                                    .filter_map(|glyph| names.get(usize::from(glyph.to_u16())))
                                    .cloned()
                                    .collect(),
                                "GSClass",
                                &mut class_definitions,
                                &mut class_serial,
                            ) else {
                                continue;
                            };
                            let mut tokens = vec![first];
                            for class in rule.input_sequence() {
                                let glyphs = (0..names.len())
                                    .filter_map(|raw_id| {
                                        let glyph = GlyphId::new(raw_id as u32);
                                        (class_def.get(glyph) == class.get())
                                            .then(|| names.get(raw_id).cloned())
                                            .flatten()
                                    })
                                    .collect();
                                let Some(token) = class_source(
                                    glyphs,
                                    "GSClass",
                                    &mut class_definitions,
                                    &mut class_serial,
                                ) else {
                                    tokens.clear();
                                    break;
                                };
                                tokens.push(token);
                            }
                            let target_index = usize::from(records[0].sequence_index());
                            let target_class = if target_index == 0 {
                                first_class
                            } else {
                                let Some(class) = rule.input_sequence().get(target_index - 1)
                                else {
                                    continue;
                                };
                                class.get()
                            };
                            if tokens.is_empty() || target_index >= tokens.len() {
                                continue;
                            }
                            let Ok(target_lookup) = lookups
                                .lookups()
                                .get(usize::from(records[0].lookup_list_index()))
                            else {
                                continue;
                            };
                            for (source, target) in imported_single_substitution_map(&target_lookup)
                            {
                                if class_def.get(GlyphId::new(u32::from(source))) != target_class {
                                    continue;
                                }
                                let Some(target_name) = names.get(usize::from(target)) else {
                                    continue;
                                };
                                tokens[target_index] =
                                    format!("{}'", tokens[target_index].trim_end_matches('\''));
                                rules.push(format!("sub {} by {target_name};", tokens.join(" ")));
                            }
                            continue;
                        }
                        if let read_fonts::tables::layout::SequenceContext::Format1(context) = table
                        {
                            let Ok(coverage) = context.coverage() else {
                                continue;
                            };
                            for (first_glyph, rule_set) in coverage
                                .iter()
                                .zip(context.seq_rule_sets().iter().flatten())
                            {
                                let Ok(rule_set) = rule_set else {
                                    continue;
                                };
                                for rule in rule_set.seq_rules().iter() {
                                    let Ok(rule) = rule else {
                                        continue;
                                    };
                                    let records = rule.seq_lookup_records();
                                    if records.len() != 1 {
                                        continue;
                                    }
                                    let Ok(target_lookup) = lookups
                                        .lookups()
                                        .get(usize::from(records[0].lookup_list_index()))
                                    else {
                                        continue;
                                    };
                                    let substitutions =
                                        imported_single_substitution_map(&target_lookup);
                                    if substitutions.is_empty() {
                                        continue;
                                    }
                                    let Some(first_name) =
                                        names.get(usize::from(first_glyph.to_u16()))
                                    else {
                                        continue;
                                    };
                                    let mut tokens = vec![first_name.clone()];
                                    for glyph in rule.input_sequence() {
                                        let Some(name) =
                                            names.get(usize::from(glyph.get().to_u16()))
                                        else {
                                            tokens.clear();
                                            break;
                                        };
                                        tokens.push(name.clone());
                                    }
                                    let target_index = usize::from(records[0].sequence_index());
                                    if tokens.is_empty() || target_index >= tokens.len() {
                                        continue;
                                    }
                                    for (source, target) in substitutions {
                                        let Some(source_name) = names.get(usize::from(source))
                                        else {
                                            continue;
                                        };
                                        let Some(target_name) = names.get(usize::from(target))
                                        else {
                                            continue;
                                        };
                                        tokens[target_index] = format!("{source_name}'");
                                        rules.push(format!(
                                            "sub {} by {target_name};",
                                            tokens.join(" ")
                                        ));
                                    }
                                }
                            }
                            continue;
                        }
                        let read_fonts::tables::layout::SequenceContext::Format3(context) = table
                        else {
                            continue;
                        };
                        let records = context.seq_lookup_records();
                        if records.len() != 1 {
                            continue;
                        }
                        let lookup_record = records[0];
                        let Ok(target_lookup) = lookups
                            .lookups()
                            .get(usize::from(lookup_record.lookup_list_index()))
                        else {
                            continue;
                        };
                        let substitutions = imported_single_substitution_map(&target_lookup);
                        if substitutions.is_empty() {
                            continue;
                        }
                        let coverages = context.coverages().iter().flatten().collect::<Vec<_>>();
                        let target_index = usize::from(lookup_record.sequence_index());
                        if target_index >= coverages.len() {
                            continue;
                        }
                        let mut context_tokens = Vec::new();
                        for coverage in coverages {
                            let Some(class) = coverage_source(
                                &coverage,
                                "GSCtx",
                                &mut class_definitions,
                                &mut class_serial,
                            ) else {
                                context_tokens.clear();
                                break;
                            };
                            context_tokens.push(class);
                        }
                        if context_tokens.is_empty() {
                            continue;
                        }
                        for (source, target) in substitutions {
                            if context
                                .coverages()
                                .iter()
                                .nth(target_index)
                                .and_then(Result::ok)
                                .is_none_or(|coverage| {
                                    coverage.get(GlyphId::new(u32::from(source))).is_none()
                                })
                            {
                                continue;
                            }
                            let (Some(source), Some(target)) = (
                                names.get(usize::from(source)),
                                names.get(usize::from(target)),
                            ) else {
                                continue;
                            };
                            context_tokens[target_index] = format!("{source}'");
                            rules.push(format!("sub {} by {target};", context_tokens.join(" ")));
                        }
                    }
                }
                SubstitutionSubtables::ChainContextual(subtables) => {
                    for table in subtables.iter().flatten() {
                        if let read_fonts::tables::layout::ChainedSequenceContext::Format2(
                            context,
                        ) = table
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
                            let class_glyphs =
                                |class_def: &read_fonts::tables::layout::ClassDef<'_>,
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
                            let class_token =
                                |class_def: &read_fonts::tables::layout::ClassDef<'_>,
                                 class: u16,
                                 prefix: &str,
                                 definitions: &mut Vec<String>,
                                 serial: &mut usize|
                                 -> Option<String> {
                                    class_source(
                                        class_glyphs(class_def, class),
                                        prefix,
                                        definitions,
                                        serial,
                                    )
                                };
                            let mut tokens = Vec::new();
                            for class in rule.backtrack_sequence() {
                                let Some(token) = class_token(
                                    &backtrack_def,
                                    class.get(),
                                    "GSChainClass",
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
                                "GSChainClass",
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
                                    "GSChainClass",
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
                                    "GSChainClass",
                                    &mut class_definitions,
                                    &mut class_serial,
                                ) else {
                                    tokens.clear();
                                    break;
                                };
                                tokens.push(token);
                            }
                            let target_index = rule.backtrack_sequence().len()
                                + usize::from(records[0].sequence_index());
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
                            if tokens.is_empty() || target_index >= tokens.len() {
                                continue;
                            }
                            let Ok(target_lookup) = lookups
                                .lookups()
                                .get(usize::from(records[0].lookup_list_index()))
                            else {
                                continue;
                            };
                            for (source, target) in imported_single_substitution_map(&target_lookup)
                            {
                                if input_def.get(GlyphId::new(u32::from(source))) != target_class {
                                    continue;
                                }
                                let Some(target_name) = names.get(usize::from(target)) else {
                                    continue;
                                };
                                tokens[target_index] =
                                    format!("{}'", tokens[target_index].trim_end_matches('\''));
                                rules.push(format!("sub {} by {target_name};", tokens.join(" ")));
                            }
                            continue;
                        }
                        if let read_fonts::tables::layout::ChainedSequenceContext::Format1(
                            context,
                        ) = table
                        {
                            let Ok(coverage) = context.coverage() else {
                                continue;
                            };
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
                                    let records = rule.seq_lookup_records();
                                    if records.len() != 1 {
                                        continue;
                                    }
                                    let Ok(target_lookup) = lookups
                                        .lookups()
                                        .get(usize::from(records[0].lookup_list_index()))
                                    else {
                                        continue;
                                    };
                                    let substitutions =
                                        imported_single_substitution_map(&target_lookup);
                                    if substitutions.is_empty() {
                                        continue;
                                    }
                                    let Some(first_name) =
                                        names.get(usize::from(first_glyph.to_u16()))
                                    else {
                                        continue;
                                    };
                                    let mut tokens = Vec::new();
                                    for glyph in rule.backtrack_sequence() {
                                        let Some(name) =
                                            names.get(usize::from(glyph.get().to_u16()))
                                        else {
                                            tokens.clear();
                                            break;
                                        };
                                        tokens.push(name.clone());
                                    }
                                    tokens.push(first_name.clone());
                                    for glyph in rule.input_sequence() {
                                        let Some(name) =
                                            names.get(usize::from(glyph.get().to_u16()))
                                        else {
                                            tokens.clear();
                                            break;
                                        };
                                        tokens.push(name.clone());
                                    }
                                    for glyph in rule.lookahead_sequence() {
                                        let Some(name) =
                                            names.get(usize::from(glyph.get().to_u16()))
                                        else {
                                            tokens.clear();
                                            break;
                                        };
                                        tokens.push(name.clone());
                                    }
                                    let target_index = rule.backtrack_sequence().len()
                                        + usize::from(records[0].sequence_index());
                                    if tokens.is_empty() || target_index >= tokens.len() {
                                        continue;
                                    }
                                    for (source, target) in substitutions {
                                        let (Some(source_name), Some(target_name)) = (
                                            names.get(usize::from(source)),
                                            names.get(usize::from(target)),
                                        ) else {
                                            continue;
                                        };
                                        tokens[target_index] = format!("{source_name}'");
                                        rules.push(format!(
                                            "sub {} by {target_name};",
                                            tokens.join(" ")
                                        ));
                                    }
                                }
                            }
                            continue;
                        }
                        let read_fonts::tables::layout::ChainedSequenceContext::Format3(context) =
                            table
                        else {
                            continue;
                        };
                        let records = context.seq_lookup_records();
                        if records.len() != 1 {
                            continue;
                        }
                        let lookup_record = records[0];
                        let Ok(target_lookup) = lookups
                            .lookups()
                            .get(usize::from(lookup_record.lookup_list_index()))
                        else {
                            continue;
                        };
                        let substitutions = imported_single_substitution_map(&target_lookup);
                        let input_coverages = context
                            .input_coverages()
                            .iter()
                            .flatten()
                            .collect::<Vec<_>>();
                        let target_index = usize::from(lookup_record.sequence_index());
                        if substitutions.is_empty() || target_index >= input_coverages.len() {
                            continue;
                        }
                        let mut tokens = Vec::new();
                        for coverage in context.backtrack_coverages().iter().flatten() {
                            let Some(class) = coverage_source(
                                &coverage,
                                "GSChainB",
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
                                "GSChainI",
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
                                "GSChainL",
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
                        let input_start = usize::from(context.backtrack_glyph_count());
                        for (source, target) in substitutions {
                            if input_coverages[target_index]
                                .get(GlyphId::new(u32::from(source)))
                                .is_none()
                            {
                                continue;
                            }
                            let (Some(source), Some(target)) = (
                                names.get(usize::from(source)),
                                names.get(usize::from(target)),
                            ) else {
                                continue;
                            };
                            tokens[input_start + target_index] = format!("{source}'");
                            rules.push(format!("sub {} by {target};", tokens.join(" ")));
                        }
                    }
                }
                _ => {}
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
