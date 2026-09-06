fn normalize_glyphs_kerning_key(value: &str, left: bool) -> String {
    let prefix = if left { "@MMK_L_" } else { "@MMK_R_" };
    value.strip_prefix(prefix).unwrap_or(value).to_string()
}
