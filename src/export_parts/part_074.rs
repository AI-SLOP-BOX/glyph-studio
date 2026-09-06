
fn extract_lookup_blocks(source: &str) -> Vec<(String, String)> {
    let mut uncommented = String::with_capacity(source.len());
    for line in source.lines() {
        uncommented.push_str(line.split('#').next().unwrap_or_default());
        uncommented.push('\n');
    }
    let source = uncommented.as_str();
    let mut blocks = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = source[cursor..].find("lookup") {
        let start = cursor + relative;
        let before_is_identifier = source[..start]
            .chars()
            .next_back()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
        let after = start + "lookup".len();
        let after_is_identifier = source[after..]
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
        if before_is_identifier || after_is_identifier {
            cursor = after;
            continue;
        }
        let tail = &source[after..];
        let Some(open) = tail.find('{') else {
            break;
        };
        let header = tail[..open].trim();
        let Some(name) = header.split_whitespace().next() else {
            cursor = after + open + 1;
            continue;
        };
        if header != name {
            cursor = after + open + 1;
            continue;
        }
        if name.is_empty()
            || !name.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
            })
        {
            cursor = after + open + 1;
            continue;
        }
        let body_start = after + open + 1;
        let mut depth = 1_i32;
        let mut end = None;
        for (index, character) in source[body_start..].char_indices() {
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
        blocks.push((
            name.to_string(),
            source[body_start..body_start + end].to_string(),
        ));
        cursor = body_start + end + 1;
    }
    blocks
}
