
/// Parse the optional GDEF mark attachment classes used by
/// `lookupflag MarkAttachmentType`. AFDKO accepts both a glyph class and a
/// named class reference here; named references have already been expanded by
/// the caller when they originate in the project's class source.
fn parse_feature_mark_attach_classes(
    source: &str,
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> BTreeMap<GlyphId16, u16> {
    let mut classes = BTreeMap::new();
    for statement in source.split(';') {
        let tokens = statement.split_whitespace().collect::<Vec<_>>();
        let Some(index) = tokens
            .iter()
            .position(|token| token.eq_ignore_ascii_case("MarkAttachClassDef"))
        else {
            continue;
        };
        let Some(class_index) = tokens[index + 1..].iter().position(|value| {
            value
                .trim_matches(|character: char| ",;[]".contains(character))
                .parse::<u16>()
                .is_ok()
        }) else {
            continue;
        };
        let class = tokens[index + 1 + class_index]
            .trim_matches(|character: char| ",;[]".contains(character))
            .parse::<u16>()
            .unwrap_or_default();
        let glyphs = clean_feature_class(&tokens[index + 1..index + 1 + class_index]);
        for glyph in glyphs {
            if let Some(&glyph_id) = glyph_ids.get(glyph.as_str()) {
                classes.insert(GlyphId16::new(glyph_id), class);
            }
        }
    }
    classes
}
