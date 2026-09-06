
fn checked_i16(value: f64, label: &str) -> Result<i16, String> {
    if !value.is_finite()
        || value < i16::MIN as f64
        || value > i16::MAX as f64
        || value.fract() != 0.0
    {
        return Err(format!("{label}がTrueTypeの範囲外です"));
    }
    Ok(value as i16)
}
