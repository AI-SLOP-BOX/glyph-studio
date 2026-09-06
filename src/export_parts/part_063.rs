
fn parse_feature_references(source: &str) -> Vec<(Tag, Tag)> {
    extract_feature_blocks(source)
        .into_iter()
        .flat_map(|(parent, body)| {
            body.split(';')
                .filter_map(move |statement| {
                    let tokens = statement.split_whitespace().collect::<Vec<_>>();
                    if tokens.first() != Some(&"feature") {
                        return None;
                    }
                    let child = tokens.get(1).and_then(|value| layout_tag(value))?;
                    Some((parent, child))
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
