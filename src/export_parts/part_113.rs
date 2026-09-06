
pub fn validate_feature_source(source: &str) -> Result<(), String> {
    let mut code = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut in_string = false;
    while let Some(character) = chars.next() {
        if !in_string && character == '#' {
            for comment_char in chars.by_ref() {
                if comment_char == '\n' {
                    code.push('\n');
                    break;
                }
            }
            continue;
        }
        if character == '"' {
            in_string = !in_string;
            code.push(' ');
            continue;
        }
        if in_string {
            code.push(' ');
        } else {
            code.push(character);
        }
    }
    if in_string {
        return Err("OpenType featureの文字列が閉じていません".into());
    }
    let mut braces = 0_i32;
    for character in code.chars() {
        match character {
            '{' => braces += 1,
            '}' => braces -= 1,
            _ => {}
        }
        if braces < 0 {
            return Err("OpenType featureの閉じ括弧が不正です".into());
        }
    }
    if braces != 0 {
        return Err("OpenType featureの括弧が閉じていません".into());
    }
    let mut declared_tags = std::collections::HashSet::new();
    let tokens: Vec<_> = code.split_whitespace().collect();
    for (index, token) in tokens.iter().enumerate() {
        if *token != "feature" {
            continue;
        }
        let line_number = code[..code.find(token).unwrap_or(0)]
            .bytes()
            .filter(|&byte| byte == b'\n')
            .count()
            + 1;
        let tag = tokens
            .get(index + 1)
            .ok_or_else(|| format!("OpenType featureのタグがありません（{}行目）", line_number))?;
        if tag.len() != 4
            || !tag
                .bytes()
                .all(|byte| byte.is_ascii() && !byte.is_ascii_control())
        {
            return Err(format!(
                "OpenType featureタグはASCII 4文字で指定してください（{}行目）",
                line_number
            ));
        }
        if !declared_tags.insert((*tag).to_string()) {
            return Err(format!(
                "OpenType feature '{}' が重複しています（{}行目）",
                tag, line_number
            ));
        }
        if !tokens[index + 2..].contains(&"{") {
            return Err(format!(
                "OpenType feature宣言に '{{' がありません（{}行目）",
                line_number
            ));
        }
    }
    let mut lookup_names = std::collections::HashSet::new();
    for (name, _) in extract_lookup_blocks(source) {
        if !lookup_names.insert(name.clone()) {
            return Err(format!("OpenType lookup '{}' が重複しています", name));
        }
    }
    for statement in code.split(';') {
        let tokens = statement.split_whitespace().collect::<Vec<_>>();
        if tokens.first() != Some(&"languagesystem") {
            continue;
        }
        if tokens.len() != 3 {
            return Err("languagesystemはScriptタグとLanguageタグを指定してください".into());
        }
        let script = tokens[1].trim_matches(|character: char| "{}".contains(character));
        let language = tokens[2].trim_matches(|character: char| "{}".contains(character));
        if script.len() != 4 || !script.is_ascii() {
            return Err(format!(
                "languagesystemのScriptタグはASCII 4文字で指定してください: {}",
                tokens[1]
            ));
        }
        if !(language.len() == 3 || language.len() == 4) || !language.is_ascii() {
            return Err(format!(
                "languagesystemのLanguageタグはASCII 3〜4文字で指定してください: {}",
                tokens[2]
            ));
        }
    }
    for (_, block) in extract_feature_blocks(source) {
        for statement in block.split(';') {
            let statement_tokens = statement.split_whitespace().collect::<Vec<_>>();
            if statement_tokens.first() != Some(&"lookup") {
                continue;
            }
            let Some(name) = statement_tokens.get(1) else {
                return Err("OpenType lookup参照に名前がありません".into());
            };
            if statement_tokens.contains(&"{") {
                continue;
            }
            if !lookup_names.contains(*name) {
                return Err(format!("OpenType lookup '{}' が未定義です", name));
            }
        }
    }
    Ok(())
}
