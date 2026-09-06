
/// Parse GDEF `Attach` records. Each record maps a glyph (or glyph class) to
/// contour point indices used by attachment-aware layout engines.
fn parse_feature_attach_points(
    source: &str,
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Option<gdef::AttachList> {
    let mut records = BTreeMap::<GlyphId16, Vec<u16>>::new();
    for statement in source.split(';') {
        let tokens = statement.split_whitespace().collect::<Vec<_>>();
        let Some(keyword_index) = tokens
            .iter()
            .position(|token| token.eq_ignore_ascii_case("Attach"))
        else {
            continue;
        };
        let Some(point_index) = tokens[keyword_index + 1..]
            .iter()
            .position(|token| {
                token
                    .trim_matches(|character: char| "<>[],".contains(character))
                    .parse::<u16>()
                    .is_ok()
            })
            .map(|index| keyword_index + 1 + index)
        else {
            continue;
        };
        let names = clean_feature_class(&tokens[keyword_index + 1..point_index]);
        let points = tokens[point_index..]
            .iter()
            .filter_map(|value| {
                value
                    .trim_matches(|character: char| "<>[],".contains(character))
                    .parse::<u16>()
                    .ok()
            })
            .collect::<Vec<_>>();
        if names.is_empty() || points.is_empty() {
            continue;
        }
        let mut points = points;
        points.sort_unstable();
        points.dedup();
        for name in names {
            if let Some(&glyph_id) = glyph_ids.get(name.as_str()) {
                records.insert(GlyphId16::new(glyph_id), points.clone());
            }
        }
    }
    if records.is_empty() {
        return None;
    }
    Some(gdef::AttachList::new(
        records.keys().copied().collect(),
        records.into_values().map(gdef::AttachPoint::new).collect(),
    ))
}
