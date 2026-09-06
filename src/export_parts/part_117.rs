
fn font_vendor_id(value: &str) -> [u8; 4] {
    let bytes = value.as_bytes();
    if bytes.len() == 4 && bytes.iter().all(|byte| byte.is_ascii()) {
        [bytes[0], bytes[1], bytes[2], bytes[3]]
    } else {
        *b"GLYP"
    }
}
