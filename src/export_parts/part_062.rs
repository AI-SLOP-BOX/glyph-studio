
fn feature_uses_extension_lookups(source: &str) -> bool {
    source
        .split(|character: char| character.is_whitespace() || character == ';')
        .any(|token| token.eq_ignore_ascii_case("useExtension"))
}
