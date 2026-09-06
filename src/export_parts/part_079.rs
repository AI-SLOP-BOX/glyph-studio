
fn parse_feature_number(value: &str) -> Option<u16> {
    let value = value.trim_matches(|character: char| "<>,".contains(character));
    if let Some(hex) = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        u16::from_str_radix(hex, 16).ok()
    } else {
        value.parse().ok()
    }
}
