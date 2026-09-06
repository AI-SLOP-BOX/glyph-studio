
fn parse_mark_glyph_sets(
    source: &str,
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> BTreeMap<String, (u16, layout::CoverageTable)> {
    let mut sets = BTreeMap::new();
    let mut next_index = 0_u16;
    for statement in source.split(';') {
        let Some((raw_name, raw_values)) = statement.split_once('=') else {
            continue;
        };
        let name = raw_name.trim();
        let values = raw_values.trim();
        if !name.starts_with('@') || !values.starts_with('[') || !values.ends_with(']') {
            continue;
        }
        let glyphs = values[1..values.len() - 1]
            .split_whitespace()
            .filter_map(|value| glyph_ids.get(value.trim_matches(|c: char| ",[]".contains(c))))
            .copied()
            .map(GlyphId16::new)
            .collect::<Vec<_>>();
        if glyphs.is_empty() || sets.contains_key(name) {
            continue;
        }
        sets.insert(name.to_string(), (next_index, glyphs.into_iter().collect()));
        next_index = next_index.saturating_add(1);
    }
    sets
}
