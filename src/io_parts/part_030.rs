
/// Restores the simple, lossless GSUB lookup forms that are commonly found in
/// production fonts as editable Feature File rules. More complex lookups stay
/// in `preserved_tables` until their full semantic editor is available.
fn import_simple_gsub_features(face: &ttf_parser::Face<'_>, names: &[String]) -> String {
    let Some(gsub) = face.tables().gsub else {
        return String::new();
    };
    let mut features = std::collections::BTreeMap::<String, Vec<String>>::new();
    let mut class_definitions = Vec::new();
    let mut class_serial = 0_usize;
    for feature in gsub.features {
        let tag = feature.tag.to_string();
        if tag.len() != 4 || !tag.is_ascii() {
            continue;
        }
        let rules = features.entry(tag).or_default();
        if let Some(first_lookup_index) = feature.lookup_indices.get(0) {
            if let Some(lookup) = gsub.lookups.get(first_lookup_index) {
                if let Some(flags) = imported_lookup_flag_source!(lookup) {
                    rules.push(flags);
                }
            }
        }
        for lookup_index in feature.lookup_indices {
            let Some(lookup) = gsub.lookups.get(lookup_index) else {
                continue;
            };
            for subtable in lookup
                .subtables
                .into_iter::<ttf_parser::gsub::SubstitutionSubtable>()
            {
                match subtable {
                    ttf_parser::gsub::SubstitutionSubtable::Single(
                        ttf_parser::gsub::SingleSubstitution::Format1 { coverage, delta },
                    ) => {
                        for raw_id in 0..names.len() {
                            let source = ttf_parser::GlyphId(raw_id as u16);
                            if coverage.get(source).is_none() {
                                continue;
                            }
                            let target = u16::try_from(i32::from(source.0) + i32::from(delta))
                                .ok()
                                .map(ttf_parser::GlyphId);
                            let (Some(source), Some(target)) = (
                                feature_glyph_name(names, source),
                                target.and_then(|id| feature_glyph_name(names, id)),
                            ) else {
                                continue;
                            };
                            rules.push(format!("sub {source} by {target};"));
                        }
                    }
                    ttf_parser::gsub::SubstitutionSubtable::Single(
                        ttf_parser::gsub::SingleSubstitution::Format2 {
                            coverage,
                            substitutes,
                        },
                    ) => {
                        for raw_id in 0..names.len() {
                            let source = ttf_parser::GlyphId(raw_id as u16);
                            let Some(index) = coverage.get(source) else {
                                continue;
                            };
                            let (Some(source), Some(target)) = (
                                feature_glyph_name(names, source),
                                substitutes
                                    .get(index)
                                    .and_then(|id| feature_glyph_name(names, id)),
                            ) else {
                                continue;
                            };
                            rules.push(format!("sub {source} by {target};"));
                        }
                    }
                    ttf_parser::gsub::SubstitutionSubtable::Multiple(table) => {
                        for raw_id in 0..names.len() {
                            let source = ttf_parser::GlyphId(raw_id as u16);
                            let Some(index) = table.coverage.get(source) else {
                                continue;
                            };
                            let Some(sequence) = table.sequences.get(index) else {
                                continue;
                            };
                            let Some(source) = feature_glyph_name(names, source) else {
                                continue;
                            };
                            let targets = sequence
                                .substitutes
                                .into_iter()
                                .filter_map(|id| feature_glyph_name(names, id))
                                .collect::<Vec<_>>();
                            if targets.len() == usize::from(sequence.substitutes.len())
                                && !targets.is_empty()
                            {
                                rules.push(format!("sub {source} by [{}];", targets.join(" ")));
                            }
                        }
                    }
                    ttf_parser::gsub::SubstitutionSubtable::Alternate(table) => {
                        for raw_id in 0..names.len() {
                            let source = ttf_parser::GlyphId(raw_id as u16);
                            let Some(index) = table.coverage.get(source) else {
                                continue;
                            };
                            let Some(alternates) = table.alternate_sets.get(index) else {
                                continue;
                            };
                            let Some(source) = feature_glyph_name(names, source) else {
                                continue;
                            };
                            let targets = alternates
                                .alternates
                                .into_iter()
                                .filter_map(|id| feature_glyph_name(names, id))
                                .collect::<Vec<_>>();
                            if targets.len() == usize::from(alternates.alternates.len())
                                && !targets.is_empty()
                            {
                                rules.push(format!("sub {source} from [{}];", targets.join(" ")));
                            }
                        }
                    }
                    ttf_parser::gsub::SubstitutionSubtable::Ligature(table) => {
                        for raw_id in 0..names.len() {
                            let source = ttf_parser::GlyphId(raw_id as u16);
                            let Some(index) = table.coverage.get(source) else {
                                continue;
                            };
                            let Some(set) = table.ligature_sets.get(index) else {
                                continue;
                            };
                            let Some(source) = feature_glyph_name(names, source) else {
                                continue;
                            };
                            for ligature in set {
                                let Some(target) = feature_glyph_name(names, ligature.glyph) else {
                                    continue;
                                };
                                let components = ligature
                                    .components
                                    .into_iter()
                                    .filter_map(|id| feature_glyph_name(names, id))
                                    .collect::<Vec<_>>();
                                if components.len() == usize::from(ligature.components.len()) {
                                    rules.push(format!(
                                        "sub {source} {} by {target};",
                                        components.join(" ")
                                    ));
                                }
                            }
                        }
                    }
                    ttf_parser::gsub::SubstitutionSubtable::ReverseChainSingle(table) => {
                        for raw_id in 0..names.len() {
                            let source_id = ttf_parser::GlyphId(raw_id as u16);
                            let Some(coverage_index) = table.coverage.get(source_id) else {
                                continue;
                            };
                            let (Some(source), Some(target)) = (
                                feature_glyph_name(names, source_id),
                                table
                                    .substitutes
                                    .get(coverage_index)
                                    .and_then(|id| feature_glyph_name(names, id)),
                            ) else {
                                continue;
                            };
                            let backtrack = table
                                .backtrack_coverages
                                .into_iter()
                                .filter_map(|coverage| {
                                    imported_coverage_class!(
                                        coverage,
                                        names,
                                        class_definitions,
                                        class_serial,
                                        "GSRevB"
                                    )
                                })
                                .collect::<Vec<_>>();
                            let lookahead = table
                                .lookahead_coverages
                                .into_iter()
                                .filter_map(|coverage| {
                                    imported_coverage_class!(
                                        coverage,
                                        names,
                                        class_definitions,
                                        class_serial,
                                        "GSRevL"
                                    )
                                })
                                .collect::<Vec<_>>();
                            let mut groups = backtrack;
                            groups.push(format!("{source}'"));
                            groups.extend(lookahead);
                            rules.push(format!("reversesub {} by {target};", groups.join(" ")));
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
