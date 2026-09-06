
fn rewrite_feature_glyph_name(source: &str, old_name: &str, new_name: &str) -> String {
    let mut output = String::with_capacity(source.len());
    let mut token = String::new();
    let mut skip_next_tag = false;
    let mut previous_token = String::new();
    let flush = |output: &mut String,
                 token: &mut String,
                 skip_next_tag: &mut bool,
                 previous_token: &mut String| {
        if !token.is_empty() {
            let skip_rewrite = *skip_next_tag || previous_token == "feature";
            if token == old_name && !skip_rewrite {
                output.push_str(new_name);
            } else {
                output.push_str(token);
            }
            *skip_next_tag = false;
            *previous_token = token.clone();
            if token == "feature" {
                *skip_next_tag = true;
            }
            token.clear();
        }
    };
    for character in source.chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-') {
            token.push(character);
        } else {
            flush(
                &mut output,
                &mut token,
                &mut skip_next_tag,
                &mut previous_token,
            );
            if character == '}' {
                skip_next_tag = true;
            }
            output.push(character);
        }
    }
    flush(
        &mut output,
        &mut token,
        &mut skip_next_tag,
        &mut previous_token,
    );
    output
}
