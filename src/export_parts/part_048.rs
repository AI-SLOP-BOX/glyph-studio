
/// 指定した2マスター間で、全グリフを補間できるか確認する。
pub fn validate_interpolation(
    project: &FontProject,
    from_master_id: &str,
    to_master_id: &str,
) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    if from_master_id == to_master_id {
        issues.push(ValidationIssue {
            message: "始点と終点に同じマスターは指定できません".into(),
            glyph_name: None,
        });
        return issues;
    }
    if !project
        .masters
        .iter()
        .any(|master| master.id == from_master_id)
    {
        issues.push(ValidationIssue {
            message: format!("マスター '{}' がありません", from_master_id),
            glyph_name: None,
        });
    }
    if !project
        .masters
        .iter()
        .any(|master| master.id == to_master_id)
    {
        issues.push(ValidationIssue {
            message: format!("マスター '{}' がありません", to_master_id),
            glyph_name: None,
        });
    }
    if !issues.is_empty() {
        return issues;
    }
    for glyph in project.glyphs.values() {
        match glyph
            .layers
            .get(from_master_id)
            .zip(glyph.layers.get(to_master_id))
        {
            None => issues.push(ValidationIssue {
                message: "対応する補間レイヤーがありません".into(),
                glyph_name: Some(glyph.name.clone()),
            }),
            Some((from_layer, to_layer)) => {
                if let Some(reason) = interpolation_mismatch_reason(from_layer, to_layer) {
                    issues.push(ValidationIssue {
                        message: reason,
                        glyph_name: Some(glyph.name.clone()),
                    });
                }
            }
        }
    }
    issues
}
