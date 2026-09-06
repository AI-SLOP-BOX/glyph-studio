
fn build_simple_gsub_with_variations_and_unicode(
    source: &str,
    glyph_ids: &std::collections::HashMap<&str, u16>,
    conditional_substitutions: &[ConditionalSubstitution],
    axis_bounds: &AxisBounds,
    unicode_by_glyph: &BTreeMap<String, u32>,
) -> Option<Vec<u8>> {
    let source = normalize_feature_keywords(source);
    let expanded_source = expand_named_feature_lookups(&expand_named_feature_classes(&source));
    let raw_feature_blocks = extract_feature_blocks(&expanded_source);
    let lookup_mark_sets = extract_feature_blocks(&source)
        .iter()
        .filter_map(|(tag, block)| parse_lookup_mark_filtering_set(block).map(|name| (*tag, name)))
        .collect::<BTreeMap<_, _>>();
    let mark_sets = parse_mark_glyph_sets(&source, glyph_ids);
    let feature_blocks = raw_feature_blocks.clone();
    let mut feature_tags = feature_blocks
        .iter()
        .map(|(tag, _)| *tag)
        .collect::<Vec<_>>();
    feature_tags.sort_by_key(|tag| tag.to_be_bytes());
    feature_tags.dedup();
    if !conditional_substitutions.is_empty() {
        feature_tags.push(Tag::new(b"rvrn"));
        feature_tags.sort_by_key(|tag| tag.to_be_bytes());
        feature_tags.dedup();
    }
    if feature_tags.is_empty() {
        feature_tags.push(Tag::new(b"liga"));
    }
    let rule_sources = if feature_blocks.is_empty() {
        vec![(Tag::new(b"liga"), expanded_source.clone())]
    } else {
        feature_blocks
    };
    let lookup_flags = rule_sources
        .iter()
        .map(|(tag, block)| (*tag, parse_lookup_flags(block)))
        .collect::<BTreeMap<_, _>>();
    let mut rules = GsubRuleSet::default();
    for substitution in conditional_substitutions {
        let (Some(&base), Some(&alternate)) = (
            glyph_ids.get(substitution.base.as_str()),
            glyph_ids.get(substitution.alternate.as_str()),
        ) else {
            continue;
        };
        rules.substitutions.push((
            Tag::new(b"rvrn"),
            GlyphId16::new(base),
            GlyphId16::new(alternate),
        ));
    }
    for (rule_tag, rule_source) in rule_sources {
        for statement in rule_source.split(';') {
            let tokens: Vec<_> = statement.split_whitespace().collect();
            if tokens.first() == Some(&"ignore") && tokens.get(1) == Some(&"sub") {
                if let Some(sequence) = parse_feature_sequence(&tokens[2..], glyph_ids) {
                    rules.ignored_contexts.push((rule_tag, sequence));
                }
                continue;
            }
            if let Some(reverse_index) = tokens.iter().position(|token| *token == "reversesub") {
                let reverse_tokens = &tokens[reverse_index + 1..];
                let Some(by_index) = reverse_tokens.iter().position(|token| *token == "by") else {
                    continue;
                };
                let Some((groups, target_index)) =
                    parse_feature_groups(&reverse_tokens[..by_index], glyph_ids)
                else {
                    continue;
                };
                let replacement = clean_feature_class(&reverse_tokens[by_index + 1..])
                    .into_iter()
                    .filter_map(|name| glyph_ids.get(name.as_str()).copied())
                    .map(GlyphId16::new)
                    .collect::<Vec<_>>();
                let Some(targets) = groups.get(target_index) else {
                    continue;
                };
                if replacement.len() != targets.len() {
                    continue;
                }
                let target = targets.clone();
                let backtrack = groups[..target_index].iter().rev().cloned().collect();
                let lookahead = groups[target_index + 1..].to_vec();
                rules
                    .reverse_contexts
                    .push((rule_tag, target, backtrack, lookahead, replacement));
                continue;
            }
            let alternate_tokens = tokens
                .iter()
                .position(|token| *token == "sub")
                .and_then(|index| tokens.get(index..))
                .filter(|tokens| tokens.len() >= 4 && tokens[2] == "from");
            if let Some(tokens) = alternate_tokens {
                let Some(&target_id) = glyph_ids.get(tokens[1]) else {
                    continue;
                };
                let names = tokens[3..].join(" ");
                let names = names.trim_start_matches('[').trim_end_matches(']');
                let Some(alts) = names
                    .split_whitespace()
                    .map(|name| glyph_ids.get(name).copied().map(GlyphId16::new))
                    .collect::<Option<Vec<_>>>()
                else {
                    continue;
                };
                if !alts.is_empty() {
                    rules
                        .alternates
                        .push((rule_tag, GlyphId16::new(target_id), alts));
                }
                continue;
            }
            if let Some(sub_index) = tokens.iter().position(|token| *token == "sub") {
                let sub_tokens = &tokens[sub_index..];
                if let Some(by_index) = sub_tokens.iter().position(|token| *token == "by") {
                    let from = clean_feature_class(&sub_tokens[1..by_index]);
                    let to = clean_feature_class(&sub_tokens[by_index + 1..]);
                    if from.len() > 1 && from.len() == to.len() {
                        for (source, replacement) in from.into_iter().zip(to) {
                            if let (Some(&source_id), Some(&replacement_id)) = (
                                glyph_ids.get(source.as_str()),
                                glyph_ids.get(replacement.as_str()),
                            ) {
                                rules.substitutions.push((
                                    rule_tag,
                                    GlyphId16::new(source_id),
                                    GlyphId16::new(replacement_id),
                                ));
                            }
                        }
                        continue;
                    }
                }
            }
            if let Some(sub_index) = tokens.iter().position(|token| *token == "sub") {
                let sub_tokens = &tokens[sub_index..];
                if sub_tokens.len() < 4 {
                    continue;
                }
                let Some(by_index) = sub_tokens.iter().position(|token| *token == "by") else {
                    continue;
                };
                if by_index < 2 || by_index + 1 >= sub_tokens.len() {
                    continue;
                }
                if by_index > 2 {
                    let replacement_names = clean_feature_class(&sub_tokens[by_index + 1..]);
                    let replacement_ids = replacement_names
                        .iter()
                        .map(|name| glyph_ids.get(name.as_str()).copied().map(GlyphId16::new))
                        .collect::<Option<Vec<_>>>();
                    let parsed = parse_context_sequences(&sub_tokens[1..by_index], glyph_ids);
                    if let Some(replacement_ids) = replacement_ids {
                        for (sequence, target_index, target_choice) in parsed.iter().cloned() {
                            let replacement = if replacement_ids.len() == 1 {
                                replacement_ids[0]
                            } else {
                                *replacement_ids
                                    .get(target_choice)
                                    .unwrap_or(&replacement_ids[0])
                            };
                            rules
                                .contexts
                                .push((rule_tag, sequence, target_index, replacement));
                        }
                        if !parsed.is_empty() {
                            continue;
                        }
                    }
                }
                let first_name = sub_tokens[1].trim_end_matches('\'');
                let Some(&first) = glyph_ids.get(first_name) else {
                    continue;
                };
                if by_index == 2
                    && sub_tokens[by_index + 1..].iter().all(|name| {
                        name.trim_matches(|character: char| "[]".contains(character)) == "NULL"
                    })
                {
                    rules
                        .multiples
                        .push((rule_tag, GlyphId16::new(first), Vec::new()));
                    continue;
                }
                let Some(replacements) = sub_tokens[by_index + 1..]
                    .iter()
                    .map(|name| {
                        let name = name.trim_matches(|character: char| "[]".contains(character));
                        glyph_ids.get(name).copied().map(GlyphId16::new)
                    })
                    .collect::<Option<Vec<_>>>()
                else {
                    continue;
                };
                if by_index == 2 && replacements.len() > 1 {
                    rules
                        .multiples
                        .push((rule_tag, GlyphId16::new(first), replacements));
                    continue;
                }
                let Some(replacement) = replacements.first().copied() else {
                    continue;
                };
                let Some(components) = sub_tokens[2..by_index]
                    .iter()
                    .map(|name| {
                        let name = name.trim_matches(|character: char| "[]".contains(character));
                        glyph_ids.get(name).copied().map(GlyphId16::new)
                    })
                    .collect::<Option<Vec<_>>>()
                else {
                    continue;
                };
                if components.is_empty() {
                    rules
                        .substitutions
                        .push((rule_tag, GlyphId16::new(first), replacement));
                    continue;
                }
                rules
                    .ligatures
                    .push((rule_tag, GlyphId16::new(first), components, replacement));
                continue;
            }
            for window in tokens.windows(4) {
                if window[0] == "sub" && window[2] == "by" {
                    let Some(&target_id) = glyph_ids.get(window[1]) else {
                        continue;
                    };
                    let Some(&replacement_id) = glyph_ids.get(window[3]) else {
                        continue;
                    };
                    rules.substitutions.push((
                        rule_tag,
                        GlyphId16::new(target_id),
                        GlyphId16::new(replacement_id),
                    ));
                }
            }
        }
    }
    // Glyphs and FontLab commonly expose an automatic `aalt` feature even
    // when the author only defined per-feature alternates. Synthesize it from
    // one-to-one substitutions while preserving an explicit `aalt` feature.
    let aalt_tag = Tag::new(b"aalt");
    if !feature_tags.contains(&aalt_tag) {
        let mut alternatives = BTreeMap::<GlyphId16, Vec<GlyphId16>>::new();
        for (tag, source, replacement) in &rules.substitutions {
            if !is_aalt_source_feature(*tag) {
                continue;
            }
            let entries = alternatives.entry(*source).or_default();
            if !entries.contains(replacement) {
                entries.push(*replacement);
            }
        }
        for (source, replacements) in alternatives {
            if !replacements.is_empty() {
                rules.alternates.push((aalt_tag, source, replacements));
            }
        }
        if rules.alternates.iter().any(|(tag, _, _)| *tag == aalt_tag) {
            feature_tags.push(aalt_tag);
            feature_tags.sort_by_key(|tag| tag.to_be_bytes());
            feature_tags.dedup();
        }
    }
    if rules.substitutions.is_empty()
        && rules.multiples.is_empty()
        && rules.alternates.is_empty()
        && rules.ligatures.is_empty()
        && rules.contexts.is_empty()
        && rules.ignored_contexts.is_empty()
        && rules.reverse_contexts.is_empty()
    {
        return None;
    }
    rules
        .substitutions
        .sort_by_key(|(_, target, _)| target.to_u16());
    rules
        .ligatures
        .sort_by_key(|(_, target, _, _)| target.to_u16());
    let mut lookups = Vec::new();
    let mut feature_indices_by_tag = BTreeMap::<Tag, Vec<u16>>::new();
    for tag in &feature_tags {
        let lookup_flag = lookup_flags
            .get(tag)
            .copied()
            .unwrap_or_else(layout::LookupFlag::empty);
        let substitutions = rules
            .substitutions
            .iter()
            .filter(|(rule_tag, _, _)| rule_tag == tag)
            .collect::<Vec<_>>();
        if substitutions.is_empty() {
            continue;
        }
        let lookup = layout::Lookup::new(
            lookup_flag,
            vec![gsub::SingleSubst::format_2(
                rules
                    .substitutions
                    .iter()
                    .filter(|(rule_tag, _, _)| rule_tag == tag)
                    .map(|(_, target, _)| *target)
                    .collect(),
                rules
                    .substitutions
                    .iter()
                    .filter(|(rule_tag, _, _)| rule_tag == tag)
                    .map(|(_, _, replacement)| *replacement)
                    .collect(),
            )],
        );
        let lookup = apply_lookup_mark_set(lookup, *tag, &lookup_mark_sets, &mark_sets);
        feature_indices_by_tag
            .entry(*tag)
            .or_default()
            .push(lookups.len() as u16);
        lookups.push(gsub::SubstitutionLookup::Single(lookup));
    }
    for tag in &feature_tags {
        let lookup_flag = lookup_flags
            .get(tag)
            .copied()
            .unwrap_or_else(layout::LookupFlag::empty);
        let mut multiples = rules
            .multiples
            .iter()
            .filter(|(rule_tag, _, _)| rule_tag == tag)
            .collect::<Vec<_>>();
        if multiples.is_empty() {
            continue;
        }
        multiples.sort_by_key(|(_, target, _)| target.to_u16());
        let coverage: layout::CoverageTable =
            multiples.iter().map(|(_, target, _)| *target).collect();
        let sequences = multiples
            .iter()
            .map(|(_, _, replacements)| gsub::Sequence::new((*replacements).clone()))
            .collect();
        let lookup = layout::Lookup::new(
            lookup_flag,
            vec![gsub::MultipleSubstFormat1::new(coverage, sequences)],
        );
        let lookup = apply_lookup_mark_set(lookup, *tag, &lookup_mark_sets, &mark_sets);
        feature_indices_by_tag
            .entry(*tag)
            .or_default()
            .push(lookups.len() as u16);
        lookups.push(gsub::SubstitutionLookup::Multiple(lookup));
    }
    for tag in &feature_tags {
        let lookup_flag = lookup_flags
            .get(tag)
            .copied()
            .unwrap_or_else(layout::LookupFlag::empty);
        let mut alternates = rules
            .alternates
            .iter()
            .filter(|(rule_tag, _, _)| rule_tag == tag)
            .collect::<Vec<_>>();
        if alternates.is_empty() {
            continue;
        }
        alternates.sort_by_key(|(_, target, _)| target.to_u16());
        let coverage: layout::CoverageTable =
            alternates.iter().map(|(_, target, _)| *target).collect();
        let sets = alternates
            .iter()
            .map(|(_, _, alternatives)| gsub::AlternateSet::new((*alternatives).clone()))
            .collect();
        let lookup = layout::Lookup::new(
            lookup_flag,
            vec![gsub::AlternateSubstFormat1::new(coverage, sets)],
        );
        let lookup = apply_lookup_mark_set(lookup, *tag, &lookup_mark_sets, &mark_sets);
        feature_indices_by_tag
            .entry(*tag)
            .or_default()
            .push(lookups.len() as u16);
        lookups.push(gsub::SubstitutionLookup::Alternate(lookup));
    }
    for tag in &feature_tags {
        let lookup_flag = lookup_flags
            .get(tag)
            .copied()
            .unwrap_or_else(layout::LookupFlag::empty);
        let mut grouped = std::collections::BTreeMap::<GlyphId16, Vec<_>>::new();
        for (rule_tag, first, components, replacement) in rules.ligatures.iter() {
            if rule_tag != tag {
                continue;
            }
            grouped
                .entry(*first)
                .or_default()
                .push((components.clone(), *replacement));
        }
        if grouped.is_empty() {
            continue;
        }
        let coverage: layout::CoverageTable = grouped.keys().copied().collect();
        let sets = grouped
            .into_values()
            .map(|items| {
                gsub::LigatureSet::new(
                    items
                        .into_iter()
                        .map(|(components, replacement)| {
                            gsub::Ligature::new(replacement, components)
                        })
                        .collect(),
                )
            })
            .collect();
        let lookup = layout::Lookup::new(
            lookup_flag,
            vec![gsub::LigatureSubstFormat1::new(coverage, sets)],
        );
        let lookup = apply_lookup_mark_set(lookup, *tag, &lookup_mark_sets, &mark_sets);
        feature_indices_by_tag
            .entry(*tag)
            .or_default()
            .push(lookups.len() as u16);
        lookups.push(gsub::SubstitutionLookup::Ligature(lookup));
    }
    for (rule_tag, sequence, target_index, replacement) in rules.contexts {
        let Some(target) = sequence.get(target_index).copied() else {
            continue;
        };
        let single_lookup_index = lookups.len() as u16;
        let single = layout::Lookup::new(
            layout::LookupFlag::empty(),
            vec![gsub::SingleSubst::format_2(
                vec![target].into(),
                vec![replacement],
            )],
        );
        lookups.push(gsub::SubstitutionLookup::Single(single));
        let context = layout::Lookup::new(
            lookup_flags
                .get(&rule_tag)
                .copied()
                .unwrap_or_else(layout::LookupFlag::empty),
            vec![gsub::SubstitutionSequenceContext::from(
                layout::SequenceContext::format_3(
                    sequence
                        .into_iter()
                        .map(|glyph| std::iter::once(glyph).collect())
                        .collect(),
                    vec![layout::SequenceLookupRecord::new(
                        target_index as u16,
                        single_lookup_index,
                    )],
                ),
            )],
        );
        let context = apply_lookup_mark_set(context, rule_tag, &lookup_mark_sets, &mark_sets);
        feature_indices_by_tag
            .entry(rule_tag)
            .or_default()
            .push(lookups.len() as u16);
        lookups.push(gsub::SubstitutionLookup::Contextual(context));
    }
    for (rule_tag, sequence) in rules.ignored_contexts {
        let context = layout::Lookup::new(
            lookup_flags
                .get(&rule_tag)
                .copied()
                .unwrap_or_else(layout::LookupFlag::empty),
            vec![gsub::SubstitutionChainContext::from(
                layout::ChainedSequenceContext::format_3(
                    Vec::new(),
                    sequence.into_iter().map(Into::into).collect(),
                    Vec::new(),
                    Vec::new(),
                ),
            )],
        );
        let context = apply_lookup_mark_set(context, rule_tag, &lookup_mark_sets, &mark_sets);
        feature_indices_by_tag
            .entry(rule_tag)
            .or_default()
            .push(lookups.len() as u16);
        lookups.push(gsub::SubstitutionLookup::ChainContextual(context));
    }
    for (rule_tag, target, backtrack, lookahead, replacement) in rules.reverse_contexts {
        let lookup = layout::Lookup::new(
            lookup_flags
                .get(&rule_tag)
                .copied()
                .unwrap_or_else(layout::LookupFlag::empty),
            vec![gsub::ReverseChainSingleSubstFormat1::new(
                target.clone().into(),
                backtrack.into_iter().map(Into::into).collect(),
                lookahead.into_iter().map(Into::into).collect(),
                replacement,
            )],
        );
        let lookup = apply_lookup_mark_set(lookup, rule_tag, &lookup_mark_sets, &mark_sets);
        feature_indices_by_tag
            .entry(rule_tag)
            .or_default()
            .push(lookups.len() as u16);
        lookups.push(gsub::SubstitutionLookup::Reverse(lookup));
    }
    let feature_references = parse_feature_references(&source);
    loop {
        let mut changed = false;
        for (parent, child) in &feature_references {
            let child_indices = feature_indices_by_tag
                .get(child)
                .cloned()
                .unwrap_or_default();
            let parent_indices = feature_indices_by_tag.entry(*parent).or_default();
            for index in child_indices {
                if !parent_indices.contains(&index) {
                    parent_indices.push(index);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let lookups = if feature_uses_extension_lookups(&source) {
        lookups
            .into_iter()
            .map(wrap_gsub_extension_lookup)
            .collect()
    } else {
        lookups
    };
    let lookup_list = layout::LookupList::new(lookups);
    let rvrn_tag = Tag::new(b"rvrn");
    let rvrn_lookups = feature_indices_by_tag
        .get(&rvrn_tag)
        .cloned()
        .unwrap_or_default();
    let rvrn_feature_index = feature_tags.iter().position(|tag| *tag == rvrn_tag);
    let scripts = build_script_list(&source, &feature_tags);
    let feature_list = layout::FeatureList::new(
        feature_tags
            .into_iter()
            .map(|tag| {
                let indices = feature_indices_by_tag.remove(&tag).unwrap_or_default();
                let indices = if tag == rvrn_tag { Vec::new() } else { indices };
                layout::FeatureRecord::new(
                    tag,
                    layout::Feature::new(
                        feature_params_for_tag(tag, &source, unicode_by_glyph),
                        indices,
                    ),
                )
            })
            .collect(),
    );
    let mut table = gsub::Gsub::new(scripts, feature_list, lookup_list);
    if let Some(feature_index) = rvrn_feature_index {
        let records = conditional_substitutions
            .iter()
            .filter_map(|substitution| {
                let conditions = substitution
                    .conditions
                    .iter()
                    .filter_map(|(tag, range)| {
                        let (axis_index, min_value, default_value, max_value) =
                            *axis_bounds.get(tag).or_else(|| {
                                axis_bounds
                                    .iter()
                                    .find(|(axis, _)| axis.eq_ignore_ascii_case(tag))
                                    .map(|(_, bounds)| bounds)
                            })?;
                        let normalize = |value: f64| {
                            if value >= default_value {
                                (value - default_value) / (max_value - default_value).max(1e-9)
                            } else {
                                (value - default_value) / (default_value - min_value).max(1e-9)
                            }
                        };
                        let min = range.min.map(normalize).unwrap_or(-1.0).clamp(-1.0, 1.0);
                        let max = range.max.map(normalize).unwrap_or(1.0).clamp(-1.0, 1.0);
                        Some(layout::Condition::format_1_axis_range(
                            axis_index,
                            write_fonts::types::F2Dot14::from_f32(min as f32),
                            write_fonts::types::F2Dot14::from_f32(max as f32),
                        ))
                    })
                    .collect::<Vec<_>>();
                (!conditions.is_empty()).then(|| {
                    layout::FeatureVariationRecord::new(
                        Some(layout::ConditionSet::new(conditions)),
                        Some(layout::FeatureTableSubstitution::new(vec![
                            layout::FeatureTableSubstitutionRecord::new(
                                feature_index as u16,
                                layout::Feature::new(None, rvrn_lookups.clone()),
                            ),
                        ])),
                    )
                })
            })
            .collect::<Vec<_>>();
        if !records.is_empty() {
            table.feature_variations = Some(layout::FeatureVariations::new(records)).into();
        }
    }
    write_fonts::dump_table(&table).ok()
}
