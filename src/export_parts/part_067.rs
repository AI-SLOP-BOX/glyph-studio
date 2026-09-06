
fn parse_feature_anchor(tokens: &[&str], anchor_index: usize) -> Option<(i16, i16)> {
    let values = tokens
        .get(anchor_index + 1..)?
        .iter()
        .take_while(|token| !token.contains('>'))
        .chain(
            tokens
                .get(anchor_index + 1..)?
                .iter()
                .filter(|token| token.contains('>'))
                .take(1),
        )
        .map(|token| token.trim_matches(|character: char| "><".contains(character)))
        .filter_map(|token| token.parse::<i16>().ok())
        .collect::<Vec<_>>();
    (values.len() >= 2).then(|| (values[0], values[1]))
}
