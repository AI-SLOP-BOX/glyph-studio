fn plist_u32(value: &plist::Value) -> Option<u32> {
    if let Some(number) = value.as_signed_integer() {
        return u32::try_from(number).ok();
    }
    value
        .as_string()
        .and_then(|text| u32::from_str_radix(text.trim().trim_start_matches("0x"), 16).ok())
}
