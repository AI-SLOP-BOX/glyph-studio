
pub fn extract_feature_blocks(source: &str) -> Vec<(Tag, String)> {
    // Comments may contain words such as `feature` or unmatched braces. Remove
    // them before scanning so they cannot distort the nesting depth.
    let mut uncommented = String::with_capacity(source.len());
    for line in source.lines() {
        uncommented.push_str(line.split('#').next().unwrap_or_default());
        uncommented.push('\n');
    }
    let source = uncommented.as_str();
    let mut blocks = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = source[cursor..].find("feature") {
        let start = cursor + relative;
        let before_is_identifier = source[..start]
            .chars()
            .next_back()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
        let after = start + "feature".len();
        let after_is_identifier = source[after..]
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
        if before_is_identifier || after_is_identifier {
            cursor = after;
            continue;
        }
        let tail = &source[start + "feature".len()..];
        let mut parts = tail.splitn(2, '{');
        let Some(header) = parts.next() else {
            break;
        };
        let Some(body_start) = parts.next() else {
            break;
        };
        let tag_name = header.split_whitespace().next().unwrap_or_default();
        if tag_name.len() != 4 || !tag_name.is_ascii() {
            cursor = start + "feature".len();
            continue;
        }
        let mut depth = 1_i32;
        let mut end = None;
        for (index, character) in body_start.char_indices() {
            match character {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(index);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(end) = end else {
            break;
        };
        let tag = Tag::new(tag_name.as_bytes().try_into().unwrap());
        blocks.push((tag, body_start[..end].to_string()));
        cursor = start + "feature".len() + end + 1;
    }
    blocks
}
