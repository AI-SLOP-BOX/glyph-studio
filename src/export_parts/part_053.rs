
pub fn validate_feature_class_definitions(
    source: &str,
    glyphs: &std::collections::HashMap<String, crate::font_data::GlyphData>,
) -> Vec<String> {
    let mut issues = Vec::new();
    let mut names = std::collections::HashSet::new();
    for (index, statement) in source.split(';').enumerate() {
        let trimmed = statement.trim();
        let Some((raw_name, raw_values)) = trimmed.split_once('=') else {
            continue;
        };
        let name = raw_name.trim();
        if !name.starts_with('@') {
            continue;
        }
        if name.len() < 2
            || !name[1..]
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            issues.push(format!("OpenType Class {}個目の名前が不正です", index + 1));
        }
        if !names.insert(name.to_string()) {
            issues.push(format!("OpenType Class '{}' が重複しています", name));
        }
        let values = raw_values.trim();
        if !values.starts_with('[') || !values.ends_with(']') {
            issues.push(format!("OpenType Class '{}' は [ ] で囲んでください", name));
            continue;
        }
        for glyph_name in values[1..values.len() - 1].split_whitespace() {
            let glyph_name = glyph_name.trim_matches(|c: char| ",[]".contains(c));
            if !glyph_name.is_empty() && !glyphs.contains_key(glyph_name) {
                issues.push(format!(
                    "OpenType Class '{}' の未定義グリフ '{}'",
                    name, glyph_name
                ));
            }
        }
    }
    issues.sort();
    issues.dedup();
    issues
}
