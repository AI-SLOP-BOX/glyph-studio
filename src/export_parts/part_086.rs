
/// Expand fixed-coordinate `anchorDef` references used by Feature File
/// mark/cursive rules. Named anchors are a source convenience; the generated
/// GPOS records still use the same concrete anchor parser as inline anchors.
fn expand_named_anchors(source: &str) -> String {
    let definitions = source
        .split(';')
        .filter_map(|statement| {
            let tokens = statement.split_whitespace().collect::<Vec<_>>();
            if tokens.first() != Some(&"anchorDef") || tokens.len() < 4 {
                return None;
            }
            let name = tokens
                .last()?
                .trim_matches(|character: char| ",;".contains(character));
            if name.is_empty()
                || !name.chars().all(|character| {
                    character.is_ascii_alphanumeric() || character == '_' || character == '.'
                })
            {
                return None;
            }
            let values = tokens[1..tokens.len() - 1]
                .iter()
                .flat_map(|token| token.split(|character: char| "<>,".contains(character)))
                .filter(|value| !value.is_empty())
                .filter_map(|value| value.parse::<i16>().ok())
                .collect::<Vec<_>>();
            (values.len() >= 2).then(|| (name.to_string(), (values[0], values[1])))
        })
        .collect::<Vec<_>>();
    let mut expanded = source.to_string();
    for (name, (x, y)) in definitions {
        expanded = expanded.replace(&format!("<anchor {name}>"), &format!("<anchor {x} {y}>"));
    }
    expanded
}
