fn gradient_alpha(alpha: f64) -> u16 {
    font_types::F2Dot14::from_f32(alpha.clamp(0.0, 1.0) as f32)
        .to_bits()
        .cast_unsigned()
}
