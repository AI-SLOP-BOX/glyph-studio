
#[cfg_attr(not(test), allow(dead_code))]
fn build_kerning_gpos(
    project: &FontProject,
    glyph_ids: &std::collections::HashMap<&str, u16>,
    source: &str,
) -> Option<Vec<u8>> {
    build_kerning_gpos_with_unicode(project, glyph_ids, source, &BTreeMap::new())
}
