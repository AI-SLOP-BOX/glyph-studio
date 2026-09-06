
fn top_level_angle_groups(text: &str) -> Vec<String> {
    let mut groups = Vec::new();
    let mut depth = 0_i32;
    let mut start = None;
    for (index, character) in text.char_indices() {
        match character {
            '<' => {
                if depth == 0 {
                    start = Some(index + character.len_utf8());
                }
                depth += 1;
            }
            '>' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(start) = start.take() {
                        groups.push(text[start..index].to_string());
                    }
                }
            }
            _ => {}
        }
    }
    groups
}
