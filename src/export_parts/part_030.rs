fn gradient_stop_offset(offset: f64) -> u16 {
    font_types::F2Dot14::from_f32(offset as f32)
        .to_bits()
        .cast_unsigned()
}
