
fn parse_feature_name_records(source: &str) -> Vec<fonttools::name::NameRecord> {
    let body = extract_table_blocks(source)
        .into_iter()
        .find(|(tag, _)| tag.eq_ignore_ascii_case("name"))
        .map(|(_, body)| body)
        .unwrap_or_default();
    let mut records = Vec::new();
    for statement in body.split(';') {
        let tokens = statement.split_whitespace().collect::<Vec<_>>();
        let Some(nameid_index) = tokens
            .iter()
            .position(|token| token.eq_ignore_ascii_case("nameid"))
        else {
            continue;
        };
        let Some(quote_start) = statement.find('"') else {
            continue;
        };
        let Some(quote_end) = statement[quote_start + 1..].find('"') else {
            continue;
        };
        let value = statement[quote_start + 1..quote_start + 1 + quote_end].replace("\\\"", "\"");
        let numeric = tokens[nameid_index + 1..]
            .iter()
            .take_while(|token| !token.contains('"'))
            .filter_map(|value| parse_feature_number(value))
            .collect::<Vec<_>>();
        let Some(&name_id) = numeric.first() else {
            continue;
        };
        let record = match numeric.as_slice() {
            [_] => fonttools::name::NameRecord::windows_unicode(name_id, value),
            [_, platform] if *platform == 1 => fonttools::name::NameRecord {
                platformID: 1,
                encodingID: 0,
                languageID: 0,
                nameID: name_id,
                string: value,
            },
            [_, platform, encoding, language] => fonttools::name::NameRecord {
                platformID: *platform,
                encodingID: *encoding,
                languageID: *language,
                nameID: name_id,
                string: value,
            },
            _ => continue,
        };
        records.push(record);
    }
    records
}
