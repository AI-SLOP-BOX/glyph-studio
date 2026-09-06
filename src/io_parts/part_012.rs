fn plist_number(value: &plist::Value) -> Option<f64> {
    value
        .as_real()
        .or_else(|| value.as_signed_integer().map(|n| n as f64))
}
