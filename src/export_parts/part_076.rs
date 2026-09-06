
/// Build the standard FeatureParams payloads used by UI clients to identify
/// stylistic sets and character variants. The feature source remains the
/// source of truth for substitutions; these records only add the metadata
/// that makes `ss##`/`cv##` appear as named controls in font applications.
fn feature_params_for_tag(
    tag: Tag,
    source: &str,
    unicode_by_glyph: &BTreeMap<String, u32>,
) -> Option<layout::FeatureParams> {
    let source = normalize_feature_keywords(source);
    let bytes = tag.to_be_bytes();
    let prefix = &bytes[..2];
    if matches!(prefix, b"ss" | b"cv") && bytes[2].is_ascii_digit() && bytes[3].is_ascii_digit() {
        let number = u16::from(bytes[2] - b'0') * 10 + u16::from(bytes[3] - b'0');
        if !(1..=20).contains(&number) {
            return None;
        }
        let index = number - 1;
        if prefix == b"ss" {
            return Some(layout::FeatureParams::StylisticSet(
                layout::StylisticSetParams::new(NameId::new(500 + index)),
            ));
        }
        return Some(layout::FeatureParams::CharacterVariant(
            layout::CharacterVariantParams::new(
                NameId::new(520 + index),
                NameId::new(0),
                NameId::new(0),
                0,
                NameId::new(0),
                extract_feature_blocks(&source)
                    .into_iter()
                    .find(|(feature_tag, _)| *feature_tag == tag)
                    .into_iter()
                    .flat_map(|(_, body)| body.split(';').map(str::to_string).collect::<Vec<_>>())
                    .filter_map(|statement| {
                        let tokens = statement.split_whitespace().collect::<Vec<_>>();
                        let sub_index = tokens.iter().position(|token| *token == "sub")?;
                        let by_index = tokens[sub_index + 1..]
                            .iter()
                            .position(|token| *token == "by")?
                            + sub_index
                            + 1;
                        Some(
                            tokens[sub_index + 1..by_index]
                                .iter()
                                .filter_map(|name| {
                                    let name = name
                                        .trim_matches(|character: char| "[],'".contains(character));
                                    unicode_by_glyph.get(name).copied()
                                })
                                .collect::<Vec<_>>(),
                        )
                    })
                    .flatten()
                    .filter(|unicode| *unicode <= 0xFFFFFF)
                    .map(Uint24::new)
                    .collect(),
            ),
        ));
    }
    if bytes != *b"size" {
        return None;
    }
    let values = extract_feature_blocks(&source)
        .into_iter()
        .find(|(feature_tag, _)| *feature_tag == tag)
        .and_then(|(_, body)| {
            body.split(';')
                .map(|statement| statement.split_whitespace().collect::<Vec<_>>())
                .find(|tokens| tokens.first() == Some(&"parameters"))
                .map(|tokens| {
                    tokens
                        .into_iter()
                        .skip(1)
                        .filter_map(|value| {
                            value
                                .trim_matches(|character: char| "<>;,".contains(character))
                                .parse()
                                .ok()
                        })
                        .collect::<Vec<u16>>()
                })
        })?;
    match values.as_slice() {
        [design_size] => Some(layout::FeatureParams::Size(layout::SizeParams::new(
            *design_size,
            0,
            0,
            0,
            0,
        ))),
        [design_size, identifier, range_start, range_end] => Some(layout::FeatureParams::Size(
            layout::SizeParams::new(*design_size, *identifier, 0, *range_start, *range_end),
        )),
        [design_size, identifier, range_start, range_end, name_entry] => {
            Some(layout::FeatureParams::Size(layout::SizeParams::new(
                *design_size,
                *identifier,
                *name_entry,
                *range_start,
                *range_end,
            )))
        }
        _ => None,
    }
}
