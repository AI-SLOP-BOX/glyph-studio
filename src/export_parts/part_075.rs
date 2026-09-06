
fn expand_named_feature_lookups(source: &str) -> String {
    let lookup_blocks = extract_lookup_blocks(source)
        .into_iter()
        .collect::<BTreeMap<String, String>>();
    if lookup_blocks.is_empty() {
        return source.to_string();
    }
    let expand = |name: &str, visiting: &mut Vec<String>| -> String {
        fn expand_one(
            name: &str,
            definitions: &BTreeMap<String, String>,
            visiting: &mut Vec<String>,
        ) -> String {
            if visiting.iter().any(|current| current == name) {
                return String::new();
            }
            let Some(body) = definitions.get(name) else {
                return String::new();
            };
            visiting.push(name.to_string());
            let mut expanded = body.clone();
            for statement in body.split(';') {
                let tokens = statement.split_whitespace().collect::<Vec<_>>();
                if tokens.first() != Some(&"lookup") {
                    continue;
                }
                let Some(reference) = tokens.get(1) else {
                    continue;
                };
                expanded.push('\n');
                expanded.push_str(&expand_one(reference, definitions, visiting));
            }
            visiting.pop();
            expanded
        }
        expand_one(name, &lookup_blocks, visiting)
    };
    let mut expanded_blocks = Vec::new();
    for (tag, body) in extract_feature_blocks(source) {
        let mut merged = body.clone();
        for statement in body.split(';') {
            let tokens = statement.split_whitespace().collect::<Vec<_>>();
            if tokens.first() != Some(&"lookup") {
                continue;
            }
            let Some(name) = tokens.get(1) else {
                continue;
            };
            merged.push('\n');
            merged.push_str(&expand(name, &mut Vec::new()));
        }
        expanded_blocks.push((tag, merged));
    }
    if expanded_blocks.is_empty() {
        return source.to_string();
    }
    // Replacing only the extracted feature bodies is unnecessary for the
    // compiler: return a synthetic source whose feature blocks contain both
    // their original statements and all referenced lookup bodies.
    expanded_blocks
        .into_iter()
        .map(|(tag, body)| format!("feature {} {{\n{}\n}} {};\n", tag, body, tag))
        .collect::<String>()
}
