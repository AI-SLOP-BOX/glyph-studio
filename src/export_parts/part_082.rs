
fn build_simple_gsub_with_variations(
    source: &str,
    glyph_ids: &std::collections::HashMap<&str, u16>,
    conditional_substitutions: &[ConditionalSubstitution],
    axis_bounds: &AxisBounds,
) -> Option<Vec<u8>> {
    build_simple_gsub_with_variations_and_unicode(
        source,
        glyph_ids,
        conditional_substitutions,
        axis_bounds,
        &BTreeMap::new(),
    )
}
