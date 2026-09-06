
fn checked_u16(value: f64, label: &str) -> Result<u16, String> {
    if !value.is_finite() || value < 0.0 || value > u16::MAX as f64 || value.fract() != 0.0 {
        return Err(format!("{label}は有効な整数範囲で指定してください"));
    }
    Ok(value as u16)
}
