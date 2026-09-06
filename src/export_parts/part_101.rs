
fn parse_feature_glyph_classes(
    source: &str,
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> BTreeMap<GlyphId16, gdef::GlyphClassDef> {
    let mut classes = BTreeMap::new();
    for statement in source.split(';') {
        let Some(definition) = statement.split_once("GlyphClassDef").map(|(_, rest)| rest) else {
            continue;
        };
        for (class_index, group) in definition.split(',').take(4).enumerate() {
            let Some(open) = group.find('[') else {
                continue;
            };
            let Some(close) = group[open + 1..].find(']') else {
                continue;
            };
            let names = &group[open + 1..open + 1 + close];
            let class = match class_index {
                0 => gdef::GlyphClassDef::Base,
                1 => gdef::GlyphClassDef::Ligature,
                2 => gdef::GlyphClassDef::Mark,
                3 => gdef::GlyphClassDef::Component,
                _ => continue,
            };
            for name in names.split_whitespace() {
                let name = name.trim_matches(|character: char| "[]".contains(character));
                if let Some(&glyph_id) = glyph_ids.get(name) {
                    classes.insert(GlyphId16::new(glyph_id), class);
                }
            }
        }
    }
    classes
}
