fn glyphs_kerning_key(project: &FontProject, value: &str, left: bool) -> String {
    let is_group = project.glyphs.values().any(|glyph| {
        if left {
            glyph.left_kerning_group == value
        } else {
            glyph.right_kerning_group == value
        }
    });
    if is_group {
        format!("@MMK_{}_{}", if left { "L" } else { "R" }, value)
    } else {
        value.to_string()
    }
}
