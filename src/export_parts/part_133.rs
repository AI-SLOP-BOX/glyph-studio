
fn mac_style_flags(metadata: &FontMetadata) -> u16 {
    (style_is_bold(metadata) as u16) | ((style_is_italic(metadata) as u16) << 1)
}
