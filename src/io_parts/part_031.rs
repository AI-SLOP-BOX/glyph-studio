
fn format_gpos_value(value: ttf_parser::gpos::ValueRecord<'_>) -> Option<String> {
    if value.x_placement_device.is_some()
        || value.y_placement_device.is_some()
        || value.x_advance_device.is_some()
        || value.y_advance_device.is_some()
    {
        return None;
    }
    if value.x_placement == 0
        && value.y_placement == 0
        && value.x_advance == 0
        && value.y_advance == 0
    {
        return None;
    }
    Some(format!(
        "<{} {} {} {}>",
        value.x_placement, value.y_placement, value.x_advance, value.y_advance
    ))
}
