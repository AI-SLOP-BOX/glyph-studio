
fn parse_glyphs_position(value: &str) -> Option<(f64, f64)> {
    let cleaned = value
        .trim()
        .trim_start_matches(['{', '('])
        .trim_end_matches(['}', ')']);
    let mut parts = cleaned.split(',').map(str::trim);
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}
