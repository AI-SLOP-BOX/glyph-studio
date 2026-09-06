
fn clean_feature_class(parts: &[&str]) -> Vec<String> {
    parts
        .iter()
        .map(|part| part.trim_matches(|c: char| "[],'".contains(c)).to_string())
        .filter(|part| !part.is_empty())
        .collect()
}
