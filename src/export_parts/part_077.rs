
/// Read all display names from a Feature File `featureNames` block. The
/// generated name ID is supplied by the caller because it is determined by
/// the registered `ss##`/`cv##` tag.
fn feature_name_records(source: &str, tag: &str, name_id: u16) -> Vec<fonttools::name::NameRecord> {
    let (_, body) = extract_feature_blocks(source)
        .into_iter()
        .find(|(feature_tag, _)| String::from_utf8_lossy(&feature_tag.to_be_bytes()) == tag)
        .unwrap_or((Tag::new(b"    "), String::new()));
    let Some(names_start) = body.find("featureNames") else {
        return Vec::new();
    };
    let names = &body[names_start..];
    let mut records = Vec::new();
    for statement in names.split(';') {
        let Some(name_start) = statement.find("name") else {
            continue;
        };
        let prefix = &statement[name_start + "name".len()..];
        let Some(quote_start) = prefix.find('"') else {
            continue;
        };
        let quote_start = quote_start + 1;
        let Some(quote_end) = prefix[quote_start..].find('"') else {
            continue;
        };
        let value = prefix[quote_start..quote_start + quote_end]
            .trim()
            .replace("\\\"", "\"");
        if value.is_empty() {
            continue;
        }
        let numeric = prefix[..quote_start - 1]
            .split_whitespace()
            .filter_map(parse_feature_number)
            .collect::<Vec<_>>();
        let record = match numeric.as_slice() {
            [platform, encoding, language] => fonttools::name::NameRecord {
                platformID: *platform,
                encodingID: *encoding,
                languageID: *language,
                nameID: name_id,
                string: value,
            },
            [] => fonttools::name::NameRecord::windows_unicode(name_id, value),
            _ => continue,
        };
        records.push(record);
    }
    records
}
