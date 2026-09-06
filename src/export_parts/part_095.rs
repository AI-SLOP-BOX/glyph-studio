
fn layout_language_tag(value: &str) -> Option<Tag> {
    let value = value.trim_matches(|character: char| "{};".contains(character));
    if value.len() == 4 && value.is_ascii() {
        return Some(Tag::new(value.as_bytes().try_into().ok()?));
    }
    if value.len() == 3 && value.is_ascii() {
        let mut bytes = [b' '; 4];
        bytes[..3].copy_from_slice(value.as_bytes());
        return Some(Tag::new(&bytes));
    }
    None
}
