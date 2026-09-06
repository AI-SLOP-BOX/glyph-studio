
fn parse_feature_sequence(
    parts: &[&str],
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Option<Vec<Vec<GlyphId16>>> {
    let mut groups = Vec::<Vec<String>>::new();
    let mut current = Vec::new();
    let mut in_class = false;
    for raw in parts {
        let mut token = (*raw).to_string();
        if token.starts_with('[') {
            in_class = true;
            token = token.trim_start_matches('[').to_string();
        }
        if token.ends_with(']') {
            token = token.trim_end_matches(']').to_string();
            if !token.is_empty() {
                current.push(token);
            }
            groups.push(std::mem::take(&mut current));
            in_class = false;
        } else if in_class {
            current.push(token);
        } else if !token.is_empty() {
            groups.push(vec![token]);
        }
    }
    if in_class || groups.is_empty() {
        return None;
    }
    groups
        .into_iter()
        .map(|group| {
            group
                .into_iter()
                .map(|name| {
                    let name = name.trim_matches(|character: char| ",[]".contains(character));
                    glyph_ids.get(name).copied().map(GlyphId16::new)
                })
                .collect::<Option<Vec<_>>>()
        })
        .collect()
}
