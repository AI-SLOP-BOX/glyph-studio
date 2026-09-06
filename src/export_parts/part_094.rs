
fn layout_tag(value: &str) -> Option<Tag> {
    let value = value.trim_matches(|character: char| "{};".contains(character));
    if value.len() != 4 || !value.is_ascii() {
        return None;
    }
    let bytes: &[u8; 4] = value.as_bytes().try_into().ok()?;
    Some(Tag::new(bytes))
}
