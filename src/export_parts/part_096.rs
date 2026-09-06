
fn feature_tags_from_pair_positions(
    positions: &[(
        Tag,
        GlyphId16,
        GlyphId16,
        gpos::ValueRecord,
        gpos::ValueRecord,
    )],
) -> Vec<Tag> {
    let mut tags = positions
        .iter()
        .map(|(tag, _, _, _, _)| *tag)
        .collect::<Vec<_>>();
    tags.sort_by_key(|tag| tag.to_be_bytes());
    tags.dedup();
    tags
}
