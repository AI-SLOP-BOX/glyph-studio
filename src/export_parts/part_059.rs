
fn parse_lookup_options(source: &str) -> (layout::LookupFlag, Option<String>) {
    let tokens = source
        .split(|character: char| character.is_whitespace() || character == ';')
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    let mut flags = layout::LookupFlag::empty();
    let mut mark_filtering_set = None;
    for (index, token) in tokens.iter().enumerate() {
        match token.to_ascii_lowercase().as_str() {
            "righttoleft" => flags |= layout::LookupFlag::RIGHT_TO_LEFT,
            "ignorebaseglyphs" => flags |= layout::LookupFlag::IGNORE_BASE_GLYPHS,
            "ignoreligatures" => flags |= layout::LookupFlag::IGNORE_LIGATURES,
            "ignoremarks" => flags |= layout::LookupFlag::IGNORE_MARKS,
            "markattachmenttype" => {
                if let Some(value) = tokens.get(index + 1).and_then(|value| value.parse().ok()) {
                    flags.set_mark_attachment_class(value);
                }
            }
            "usemarkfilteringset" => {
                mark_filtering_set = tokens.get(index + 1).map(|value| (*value).to_string());
            }
            // MarkFilteringSet needs a GDEF MarkGlyphSets table. Until the
            // editor exposes named mark sets, do not emit the flag without
            // its required companion table.
            _ => {}
        }
    }
    (flags, mark_filtering_set)
}
