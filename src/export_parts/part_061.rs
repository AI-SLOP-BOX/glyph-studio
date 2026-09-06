
/// Adobe Feature File also permits the long-form `substitute` and `position`
/// keywords. Internally the compiler uses the short forms so every lookup
/// parser accepts both spellings consistently.
fn normalize_feature_keywords(source: &str) -> String {
    let tokens = source.split_whitespace().collect::<Vec<_>>();
    let mut normalized = Vec::with_capacity(tokens.len());
    let mut index = 0;
    while index < tokens.len() {
        let token = tokens[index];
        // `enum sub` and `enum pos` are Feature File's enumerated forms.
        // The compiler already expands class combinations independently, so
        // removing the marker gives both forms the same interoperable result.
        if matches!(token, "enum" | "enumerate")
            && matches!(tokens.get(index + 1), Some(&"sub") | Some(&"pos"))
        {
            index += 1;
            continue;
        }
        normalized.push(match token {
            "substitute" => "sub",
            "position" => "pos",
            "rsub" => "reversesub",
            _ => token,
        });
        index += 1;
    }
    normalized.join(" ")
}
