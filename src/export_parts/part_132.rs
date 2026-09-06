
fn style_is_bold(metadata: &FontMetadata) -> bool {
    metadata.weight_class >= 700 || metadata.style_name.to_ascii_lowercase().contains("bold")
}
