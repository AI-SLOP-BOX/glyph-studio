
fn integer_from(info: &plist::Dictionary, key: &str) -> Option<u16> {
    info.get(key)
        .and_then(|item| {
            item.as_signed_integer()
                .or_else(|| item.as_unsigned_integer().map(|v| v as i64))
        })
        .and_then(|item| u16::try_from(item).ok())
}
