
fn build_gdef(
    project: &FontProject,
    glyph_ids: &std::collections::HashMap<&str, u16>,
    feature_source: &str,
) -> Option<Vec<u8>> {
    let expanded_source = expand_named_feature_classes(feature_source);
    let explicit_classes = parse_feature_glyph_classes(&expanded_source, glyph_ids);
    let mark_attach_classes = parse_feature_mark_attach_classes(&expanded_source, glyph_ids);
    let mut records = Vec::new();
    for name in project.glyph_names_sorted() {
        let Some(&glyph_id) = glyph_ids.get(name) else {
            continue;
        };
        let Some(glyph) = project.glyphs.get(name) else {
            continue;
        };
        let anchors = project.anchors_for_glyph(name);
        let class = explicit_classes
            .get(&GlyphId16::new(glyph_id))
            .copied()
            .unwrap_or_else(|| {
                if anchors.iter().any(|anchor| anchor.name.starts_with('_')) {
                    gdef::GlyphClassDef::Mark
                } else if !glyph.components.is_empty() {
                    gdef::GlyphClassDef::Component
                } else {
                    gdef::GlyphClassDef::Base
                }
            });
        records.push(layout::ClassRangeRecord::new(
            GlyphId16::new(glyph_id),
            GlyphId16::new(glyph_id),
            class as u16,
        ));
    }
    if records.is_empty() {
        return None;
    }
    let class_def = layout::ClassDef::format_2(records);
    let mut ligature_carets = parse_feature_ligature_carets(&expanded_source, glyph_ids);
    for name in project.glyph_names_sorted() {
        let Some(&glyph_id) = glyph_ids.get(name) else {
            continue;
        };
        if ligature_carets.contains_key(&GlyphId16::new(glyph_id)) {
            continue;
        }
        let Some(glyph) = project.glyphs.get(name) else {
            continue;
        };
        if glyph.components.len() < 2 {
            continue;
        }
        let mut position = 0.0;
        let mut carets = Vec::new();
        for component in glyph.components.iter().take(glyph.components.len() - 1) {
            let component_width = project
                .glyphs
                .get(&component.base)
                .map(|base| base.width * component.x_scale + component.x_offset)
                .unwrap_or(component.x_offset);
            position += component_width;
            if let Ok(coordinate) = checked_i16(position, "合字caret位置") {
                carets.push(gdef::CaretValue::format_1(coordinate));
            }
        }
        if !carets.is_empty() {
            ligature_carets.insert(GlyphId16::new(glyph_id), carets);
        }
    }
    let ligature_caret_list = (!ligature_carets.is_empty()).then(|| {
        gdef::LigCaretList::new(
            ligature_carets.keys().copied().collect(),
            ligature_carets
                .into_values()
                .map(gdef::LigGlyph::new)
                .collect(),
        )
    });
    // Keep the named class spelling here: class expansion is useful for
    // layout rules, but GDEF MarkGlyphSets needs the original set identity so
    // lookupflag UseMarkFilteringSet can resolve to its index.
    let mark_sets = parse_mark_glyph_sets(feature_source, glyph_ids);
    let mark_glyph_sets = (!mark_sets.is_empty()).then(|| {
        let mut sets = mark_sets.values().collect::<Vec<_>>();
        sets.sort_by_key(|(index, _)| *index);
        gdef::MarkGlyphSets::new(
            sets.into_iter()
                .map(|(_, coverage)| coverage.clone())
                .collect(),
        )
    });
    let mark_attach_class_def = (!mark_attach_classes.is_empty()).then(|| {
        layout::ClassDef::format_2(
            mark_attach_classes
                .into_iter()
                .map(|(glyph, class)| layout::ClassRangeRecord::new(glyph, glyph, class))
                .collect(),
        )
    });
    let attach_list = parse_feature_attach_points(&expanded_source, glyph_ids);
    let mut table = gdef::Gdef::new(
        Some(class_def),
        attach_list,
        ligature_caret_list,
        mark_attach_class_def,
    );
    table.mark_glyph_sets_def = mark_glyph_sets.into();
    write_fonts::dump_table(&table).ok()
}
