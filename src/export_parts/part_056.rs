
fn parse_feature_groups(
    parts: &[&str],
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Option<(Vec<Vec<GlyphId16>>, usize)> {
    let mut groups = Vec::<(Vec<String>, bool)>::new();
    let mut current = Vec::new();
    let mut in_class = false;
    let mut marked = false;
    for raw in parts {
        let mut token = (*raw).to_string();
        if token.starts_with('[') {
            in_class = true;
            token = token.trim_start_matches('[').to_string();
        }
        if token.ends_with(']') || token.ends_with("]'") {
            marked |= token.ends_with("]'");
            token = token
                .trim_end_matches('\'')
                .trim_end_matches(']')
                .to_string();
            if !token.is_empty() {
                current.push(token);
            }
            groups.push((std::mem::take(&mut current), marked));
            marked = false;
            in_class = false;
        } else if in_class {
            marked |= token.ends_with('\'');
            current.push(token.trim_end_matches('\'').to_string());
        } else {
            marked = token.ends_with('\'');
            groups.push((vec![token.trim_end_matches('\'').to_string()], marked));
            marked = false;
        }
    }
    if in_class || groups.iter().filter(|(_, marked)| *marked).count() != 1 {
        return None;
    }
    let target_index = groups.iter().position(|(_, marked)| *marked)?;
    let groups = groups
        .into_iter()
        .map(|(names, _)| {
            names
                .into_iter()
                .map(|name| glyph_ids.get(name.as_str()).copied().map(GlyphId16::new))
                .collect::<Option<Vec<_>>>()
        })
        .collect::<Option<Vec<_>>>()?;
    Some((groups, target_index))
}
