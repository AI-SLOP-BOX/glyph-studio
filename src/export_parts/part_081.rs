
#[cfg_attr(not(test), allow(dead_code))]
fn build_simple_gsub(
    source: &str,
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Option<Vec<u8>> {
    build_simple_gsub_with_variations(source, glyph_ids, &[], &std::collections::HashMap::new())
}
