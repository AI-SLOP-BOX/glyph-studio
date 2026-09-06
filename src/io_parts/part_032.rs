
fn format_gpos_pair(
    first: ttf_parser::gpos::ValueRecord<'_>,
    second: ttf_parser::gpos::ValueRecord<'_>,
) -> Option<String> {
    let has_device = |value: ttf_parser::gpos::ValueRecord<'_>| {
        value.x_placement_device.is_some()
            || value.y_placement_device.is_some()
            || value.x_advance_device.is_some()
            || value.y_advance_device.is_some()
    };
    if has_device(first) || has_device(second) {
        return None;
    }
    let first = format_gpos_value(first).unwrap_or_else(|| "<0 0 0 0>".to_string());
    let second = format_gpos_value(second).unwrap_or_else(|| "<0 0 0 0>".to_string());
    (first != "<0 0 0 0>" || second != "<0 0 0 0>").then(|| format!("{first} {second}"))
}
