
fn parse_context_sequences(
    parts: &[&str],
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Vec<(Vec<GlyphId16>, usize, usize)> {
    let mut groups: Vec<(Vec<String>, bool)> = Vec::new();
    let mut class = Vec::new();
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
                class.push(token);
            }
            groups.push((std::mem::take(&mut class), marked));
            marked = false;
            in_class = false;
        } else if in_class {
            marked |= token.ends_with('\'');
            class.push(token.trim_end_matches('\'').to_string());
        } else {
            marked = token.ends_with('\'');
            groups.push((vec![token.trim_end_matches('\'').to_string()], marked));
            marked = false;
        }
    }
    if in_class || groups.iter().filter(|(_, marked)| *marked).count() != 1 {
        return Vec::new();
    }
    let target_index = groups.iter().position(|(_, marked)| *marked).unwrap();
    let alternatives: Option<Vec<Vec<GlyphId16>>> = groups
        .into_iter()
        .map(|(names, _)| {
            names
                .into_iter()
                .map(|name| glyph_ids.get(name.as_str()).copied().map(GlyphId16::new))
                .collect::<Option<Vec<_>>>()
        })
        .collect();
    let Some(alternatives) = alternatives else {
        return Vec::new();
    };
    let mut output = vec![(Vec::new(), 0usize, 0usize)];
    for (group_index, choices) in alternatives.into_iter().enumerate() {
        let mut next = Vec::new();
        for (prefix, _, target_choice) in output {
            for (choice_index, choice) in choices.iter().enumerate() {
                let mut sequence = prefix.clone();
                sequence.push(*choice);
                next.push((
                    sequence,
                    target_index,
                    if group_index == target_index {
                        choice_index
                    } else {
                        target_choice
                    },
                ));
            }
        }
        output = next;
    }
    output
}
