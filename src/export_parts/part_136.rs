
fn max_feature_context(source: &str) -> u16 {
    let mut maximum = 1_usize;
    for statement in normalize_feature_keywords(source).split(';') {
        let tokens = statement.split_whitespace().collect::<Vec<_>>();
        let Some(index) = tokens
            .iter()
            .position(|token| matches!(*token, "sub" | "pos" | "reversesub"))
        else {
            continue;
        };
        let end = tokens[index..]
            .iter()
            .position(|token| matches!(*token, "by" | "from"))
            .map(|offset| index + offset)
            .unwrap_or(tokens.len());
        maximum = maximum.max(end.saturating_sub(index + 1));
    }
    maximum.min(u16::MAX as usize) as u16
}
