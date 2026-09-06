
fn normalize_axis(value: f64, min: f64, default: f64, max: f64) -> f32 {
    if value >= default {
        if (max - default).abs() < f64::EPSILON {
            0.0
        } else {
            ((value - default) / (max - default)).clamp(-1.0, 1.0) as f32
        }
    } else if (default - min).abs() < f64::EPSILON {
        0.0
    } else {
        ((value - default) / (default - min)).clamp(-1.0, 1.0) as f32
    }
}
