
fn style_is_italic(metadata: &FontMetadata) -> bool {
    metadata.italic_angle.abs() > f64::EPSILON
        || metadata.style_name.to_ascii_lowercase().contains("italic")
        || metadata.style_name.to_ascii_lowercase().contains("oblique")
}
