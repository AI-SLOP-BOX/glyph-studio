
fn postscript_name(family: &str, style: &str) -> String {
    let sanitize = |value: &str| {
        let mut result: String = value
            .chars()
            .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
            .collect();
        while result.starts_with('-') {
            result.remove(0);
        }
        result.truncate(30);
        if result.is_empty() {
            "Font".to_string()
        } else {
            result
        }
    };
    let mut result = format!("{}-{}", sanitize(family), sanitize(style));
    result.truncate(63);
    result
}
