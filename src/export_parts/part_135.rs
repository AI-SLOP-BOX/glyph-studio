
fn os2_selection_flags(metadata: &FontMetadata) -> u16 {
    if metadata.fs_selection != 0 {
        return metadata.fs_selection;
    }
    let italic = style_is_italic(metadata);
    let bold = style_is_bold(metadata);
    let regular = !italic && !bold && metadata.weight_class == 400;
    // USE_TYPO_METRICS and WWS make modern consumers prefer the same
    // typographic metrics and family/style grouping used by the editor.
    (italic as u16) | ((bold as u16) << 5) | ((regular as u16) << 6) | (1 << 7) | (1 << 8)
}
