fn gradient_angle(degrees: f64) -> u16 {
    font_types::F2Dot14::from_f32((degrees / 180.0 - 1.0) as f32)
        .to_bits()
        .cast_unsigned()
}
