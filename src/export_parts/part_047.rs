
/// UI向けの検証結果。書き出し用の既存APIを保ったまま、対象グリフを構造化する。
pub fn validate_project_detailed(project: &FontProject) -> Vec<ValidationIssue> {
    validate_project(project)
        .into_iter()
        .map(|message| {
            let glyph_name = project.glyphs.keys().find(|name| {
                message.contains(&format!("'{}'", name))
                    || message.contains(&format!("{} の", name))
            });
            ValidationIssue {
                message,
                glyph_name: glyph_name.cloned(),
            }
        })
        .collect()
}
