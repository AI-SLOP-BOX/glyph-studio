
fn decode_name_string(platform: u16, bytes: &[u8]) -> Option<String> {
    if platform == 0 || platform == 3 {
        let (chunks, remainder) = bytes.as_chunks::<2>();
        if !remainder.is_empty() {
            return None;
        }
        let units = chunks
            .iter()
            .map(|chunk| u16::from_be_bytes(*chunk))
            .collect::<Vec<_>>();
        String::from_utf16(&units).ok()
    } else {
        None
    }
}
