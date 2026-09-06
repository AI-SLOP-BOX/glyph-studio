
pub fn expand_named_feature_classes(source: &str) -> String {
    let mut expanded = source.to_string();
    let definitions: Vec<(String, String)> = source
        .split(';')
        .filter_map(|statement| {
            let (name, values) = statement.split_once('=')?;
            let name = name.trim();
            if !name.starts_with('@') {
                return None;
            }
            let values = values
                .trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split_whitespace()
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
            (!values.is_empty()).then(|| (name.to_string(), format!("[{}]", values.join(" "))))
        })
        .collect();
    for (name, value) in definitions {
        expanded = expanded.replace(&name, &value);
    }
    expanded
}
