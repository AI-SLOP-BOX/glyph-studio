
fn checked_fixed_16_16(value: f64, label: &str) -> Result<i32, String> {
    let scaled = value * 65_536.0;
    if !scaled.is_finite() || scaled < f64::from(i32::MIN) || scaled > f64::from(i32::MAX) {
        return Err(format!("{label}が16.16固定小数点の範囲外です"));
    }
    Ok(scaled.round() as i32)
}
