
fn build_direct_kerning_lookup(
    kerning: &std::collections::HashMap<(String, String), f64>,
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Option<gpos::PositionLookup> {
    let mut grouped = BTreeMap::<GlyphId16, Vec<(GlyphId16, i16)>>::new();
    for ((left, right), value) in kerning {
        let (Some(&left_id), Some(&right_id), Ok(value)) = (
            glyph_ids.get(left.as_str()),
            glyph_ids.get(right.as_str()),
            checked_i16(*value, "可変カーニング値"),
        ) else {
            continue;
        };
        grouped
            .entry(GlyphId16::new(left_id))
            .or_default()
            .push((GlyphId16::new(right_id), value));
    }
    if grouped.is_empty() {
        return None;
    }
    let coverage: layout::CoverageTable = grouped.keys().copied().collect();
    let pair_sets = grouped
        .into_values()
        .map(|mut pairs| {
            pairs.sort_by_key(|(right, _)| right.to_u16());
            pairs.dedup_by_key(|(right, _)| *right);
            gpos::PairSet::new(
                pairs
                    .into_iter()
                    .map(|(right, value)| {
                        gpos::PairValueRecord::new(
                            right,
                            gpos::ValueRecord::new().with_x_advance(value),
                            gpos::ValueRecord::new(),
                        )
                    })
                    .collect(),
            )
        })
        .collect();
    Some(gpos::PositionLookup::Pair(layout::Lookup::new(
        layout::LookupFlag::empty(),
        vec![gpos::PairPos::format_1(coverage, pair_sets)],
    )))
}
