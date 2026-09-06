fn parse_glyphs_class_line(line: &str) -> Option<(String, String)> {
    let (name, code) = line.trim().strip_prefix('@')?.split_once('=')?;
    Some((
        name.trim().to_string(),
        code.trim()
            .trim_matches(';')
            .trim()
            .trim_matches(['[', ']'])
            .trim()
            .to_string(),
    ))
}
