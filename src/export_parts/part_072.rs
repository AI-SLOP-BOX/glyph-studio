
fn parse_feature_table_number(raw: &str) -> Option<f64> {
    let cleaned = raw.trim_matches(|character: char| "<>(),".contains(character));
    if let Some(hex) = cleaned
        .strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex, 16).ok().map(|value| value as f64);
    }
    cleaned.parse::<f64>().ok()
}
