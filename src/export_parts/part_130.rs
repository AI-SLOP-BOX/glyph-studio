
fn normalized_f2dot14(value: f64) -> i16 {
    (value.clamp(-1.0, 1.0) * 16384.0).round() as i16
}
