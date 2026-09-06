
pub fn validate_feature_glyph_references(
    source: &str,
    glyphs: &std::collections::HashMap<String, crate::font_data::GlyphData>,
) -> Vec<String> {
    let mut issues = Vec::new();
    let mut defined_classes = std::collections::HashSet::new();
    for statement in source.split(';') {
        if let Some((name, _)) = statement.split_once('=') {
            let name = name.trim();
            if name.starts_with('@') {
                defined_classes.insert(name);
            }
        }
        if statement.trim_start().starts_with("markClass ") {
            if let Some(name) = statement
                .split_whitespace()
                .rev()
                .find(|token| token.starts_with('@'))
            {
                defined_classes.insert(name);
            }
        }
    }
    let keywords = [
        "sub",
        "substitute",
        "pos",
        "position",
        "by",
        "from",
        "ignore",
        "lookup",
        "enum",
        "mark",
        "NULL",
    ];
    let mut offset = 0;
    for statement_text in source.split(';') {
        let line_number = source[..offset]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1;
        let code = statement_text
            .lines()
            .map(|line| line.split('#').next().unwrap_or_default())
            .collect::<Vec<_>>()
            .join(" ");
        let mut statement = false;
        for raw in code.split_whitespace() {
            let token = raw.trim_matches(|c: char| ";{},()[]".contains(c));
            if token == "sub" || token == "substitute" || token == "pos" || token == "position" {
                statement = true;
                continue;
            }
            if !statement || token.is_empty() || token.contains('<') || token.contains('>') {
                continue;
            }
            if token.starts_with('@') {
                if !defined_classes.contains(token) {
                    issues.push(format!(
                        "OpenType feature {}行目の未定義クラス '{}'",
                        line_number, token
                    ));
                }
                continue;
            }
            if token == "by" || token == "from" {
                continue;
            }
            if keywords.contains(&token) || token.parse::<f64>().is_ok() {
                continue;
            }
            let glyph_name = token.trim_end_matches('\'');
            if glyph_name.is_empty()
                || glyph_name.starts_with('[')
                || glyph_name.starts_with('@')
                || glyph_name == "<"
                || glyph_name == ">"
            {
                continue;
            }
            if !glyphs.contains_key(glyph_name) {
                issues.push(format!(
                    "OpenType feature {}行目の未定義グリフ '{}': 出力時に無視される可能性があります",
                    line_number,
                    glyph_name
                ));
            }
        }
        offset = offset.saturating_add(statement_text.len() + 1);
    }
    issues.sort();
    issues.dedup();
    issues
}
