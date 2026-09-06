
fn extract_table_blocks(source: &str) -> Vec<(String, String)> {
    let mut uncommented = String::with_capacity(source.len());
    for line in source.lines() {
        uncommented.push_str(line.split('#').next().unwrap_or_default());
        uncommented.push('\n');
    }
    let lower = uncommented.to_ascii_lowercase();
    let mut blocks = Vec::new();
    let mut cursor = 0;
    while let Some(relative) = lower[cursor..].find("table") {
        let start = cursor + relative;
        let before_is_identifier = lower[..start]
            .chars()
            .next_back()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
        let after = start + "table".len();
        let after_is_identifier = lower[after..]
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric() || character == '_');
        if before_is_identifier || after_is_identifier {
            cursor = after;
            continue;
        }
        let tail = &uncommented[after..];
        let Some(open) = tail.find('{') else {
            break;
        };
        let tag = tail[..open].split_whitespace().next().unwrap_or_default();
        if tag.len() != 4 || !tag.is_ascii() {
            cursor = after + open + 1;
            continue;
        }
        let body_start = after + open + 1;
        let mut depth = 1_i32;
        let mut end = None;
        for (index, character) in uncommented[body_start..].char_indices() {
            match character {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(body_start + index);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(end) = end else {
            break;
        };
        blocks.push((tag.to_string(), uncommented[body_start..end].to_string()));
        cursor = end + 1;
    }
    blocks
}
