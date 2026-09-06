
fn is_aalt_source_feature(tag: Tag) -> bool {
    let bytes = tag.to_be_bytes();
    (bytes == *b"salt" || bytes == *b"swsh" || bytes == *b"titl" || bytes == *b"ornm")
        || ((bytes[..2] == *b"ss" || bytes[..2] == *b"cv")
            && bytes[2].is_ascii_digit()
            && bytes[3].is_ascii_digit())
}
