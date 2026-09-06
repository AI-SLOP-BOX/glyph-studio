
/// Expand fixed `valueRecordDef` definitions before the GPOS parser reads
/// positioning statements. This covers the common reusable-value form; the
/// expanded values then use the same validation and ValueRecord machinery as
/// inline values.
fn expand_named_value_records(source: &str) -> String {
    let definitions = source
        .split(';')
        .filter_map(|statement| {
            let tokens = statement.split_whitespace().collect::<Vec<_>>();
            if tokens.first() != Some(&"valueRecordDef") || tokens.len() < 3 {
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
            let value_start = statement.find("valueRecordDef")? + "valueRecordDef".len();
            let value_end = statement.rfind(name)?;
            let value = statement[value_start..value_end].trim();
            (!value.is_empty()).then(|| {
                let replacement = if value.starts_with('<') && value.ends_with('>') {
                    value.to_string()
                } else {
                    // A one-number valueRecordDef is the AFM-style x/y
                    // advance shorthand, so keep it outside angle brackets.
                    value.to_string()
                };
                (name.to_string(), replacement)
            })
        })
        .collect::<Vec<_>>();
    let mut expanded = source.to_string();
    for (name, replacement) in definitions {
        expanded = expanded.replace(&format!("<{name}>"), &replacement);
    }
    expanded
}
