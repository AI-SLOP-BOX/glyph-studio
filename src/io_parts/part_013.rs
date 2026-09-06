
fn plist_bool(value: &plist::Value) -> Option<bool> {
    value
        .as_boolean()
        .or_else(|| value.as_signed_integer().map(|number| number != 0))
        .or_else(|| {
            value.as_string().and_then(|text| match text {
                "true" | "1" => Some(true),
                "false" | "0" => Some(false),
                _ => None,
            })
        })
}
