
fn real_from(info: &plist::Dictionary, key: &str) -> Option<f64> {
    info.get(key).and_then(|item| {
        item.as_real()
            .or_else(|| item.as_signed_integer().map(|v| v as f64))
    })
}
