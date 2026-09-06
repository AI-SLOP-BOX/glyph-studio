use super::*;

pub(super) fn preview_mark_attachment(
    project: &FontProject,
    base_name: &str,
    mark_name: &str,
) -> Option<(f32, f32)> {
    let mark_anchors = project.anchors_for_glyph(mark_name);
    project
        .anchors_for_glyph(base_name)
        .into_iter()
        .filter(|anchor| !anchor.name.starts_with('_'))
        .find_map(|base_anchor| {
            let mark_anchor = mark_anchors
                .iter()
                .find(|anchor| anchor.name == format!("_{}", base_anchor.name))?;
            Some((
                (base_anchor.x - mark_anchor.x) as f32,
                (base_anchor.y - mark_anchor.y) as f32,
            ))
        })
}

pub(super) fn preview_context_sequences(parts: &[&str]) -> Vec<(Vec<String>, usize)> {
    let mut groups: Vec<(Vec<String>, bool)> = Vec::new();
    let mut logical_parts = Vec::new();
    let mut index = 0;
    while index < parts.len() {
        if parts[index].starts_with('[') && !parts[index].contains(']') {
            let mut combined = parts[index].to_string();
            index += 1;
            while index < parts.len() {
                combined.push(' ');
                combined.push_str(parts[index]);
                if parts[index].contains(']') {
                    break;
                }
                index += 1;
            }
            logical_parts.push(combined);
        } else {
            logical_parts.push(parts[index].to_string());
        }
        index += 1;
    }
    for raw in &logical_parts {
        let mut token = raw.as_str();
        let marked = token.ends_with('\'');
        if marked {
            token = &token[..token.len() - 1];
        }
        if token.starts_with('[') && token.ends_with(']') {
            token = &token[1..token.len() - 1];
        }
        let choices = token
            .split_whitespace()
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        if choices.is_empty() {
            return Vec::new();
        }
        groups.push((choices, marked));
    }
    let marked_indices = groups
        .iter()
        .enumerate()
        .filter_map(|(index, (_, marked))| marked.then_some(index))
        .collect::<Vec<_>>();
    if marked_indices.len() != 1 {
        return Vec::new();
    }
    let target = marked_indices[0];
    let mut sequences = vec![(Vec::new(), target)];
    for (choices, _) in groups {
        let mut next = Vec::new();
        for (sequence, target_index) in &sequences {
            for choice in &choices {
                let mut expanded = sequence.clone();
                expanded.push(choice.clone());
                next.push((expanded, *target_index));
            }
        }
        sequences = next;
    }
    sequences
}

pub(super) fn preview_glyph_names(
    project: &FontProject,
    text: &str,
    enabled_features: &str,
) -> Vec<String> {
    let mut names: Vec<String> = text
        .chars()
        .map(|character| glyph_name_for_project_char(project, character))
        .collect();
    let mut ligatures = Vec::new();
    let mut substitutions = Vec::new();
    let mut multiples = Vec::new();
    let mut alternates = Vec::new();
    let mut contexts = Vec::new();
    let enabled: std::collections::HashSet<&str> = enabled_features
        .split([',', ' ', '\t'])
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .collect();
    let feature_source = project.feature_source();
    let expanded_features = crate::core::expand_named_feature_classes(&feature_source);
    let rule_sources = crate::core::extract_feature_blocks(&expanded_features);
    let source = if rule_sources.is_empty() {
        expanded_features
    } else {
        rule_sources
            .into_iter()
            .filter(|(tag, _)| {
                enabled.contains(std::str::from_utf8(&tag.to_be_bytes()).unwrap_or(""))
            })
            .map(|(_, body)| body)
            .collect::<Vec<_>>()
            .join(";")
    };
    for statement in source.split(';') {
        let tokens: Vec<_> = statement.split_whitespace().collect();
        let Some(sub) = tokens.iter().position(|token| *token == "sub") else {
            continue;
        };
        if sub + 2 < tokens.len() && tokens[sub + 2] == "from" {
            let from = tokens[sub + 1].trim_matches(|character: char| "[]".contains(character));
            let choices = tokens[sub + 3..]
                .iter()
                .map(|name| name.trim_matches(|character: char| "[]".contains(character)))
                .filter(|name| project.glyphs.contains_key(*name))
                .map(str::to_string)
                .collect::<Vec<_>>();
            if !from.is_empty() && !choices.is_empty() {
                alternates.push((from.to_string(), choices[0].clone()));
            }
        } else {
            let Some(by) = tokens.iter().position(|token| *token == "by") else {
                continue;
            };
            fn clean_token(token: &str) -> &str {
                token.trim_matches(|character: char| "[]'".contains(character))
            }
            let marked: Vec<_> = tokens[sub + 1..by]
                .iter()
                .enumerate()
                .filter(|(_, token)| token.ends_with('\''))
                .collect();
            if marked.len() == 1 && by > sub + 2 && by + 1 < tokens.len() {
                let replacement = clean_token(tokens[by + 1]).to_string();
                let mut parsed_context = false;
                if project.glyphs.contains_key(&replacement) {
                    for (sequence, target_index) in preview_context_sequences(&tokens[sub + 1..by])
                    {
                        if sequence
                            .iter()
                            .all(|name| project.glyphs.contains_key(name))
                        {
                            contexts.push((sequence, target_index, replacement.clone()));
                            parsed_context = true;
                        }
                    }
                }
                if parsed_context {
                    continue;
                }
            }
            if by > sub + 2 && tokens[sub + 1].starts_with('[') && tokens[by + 1].starts_with('[') {
                let from = tokens[sub + 1..by]
                    .iter()
                    .map(|token| clean_token(token).to_string())
                    .collect::<Vec<_>>();
                let to = tokens[by + 1..]
                    .iter()
                    .map(|token| clean_token(token).to_string())
                    .collect::<Vec<_>>();
                if from.len() == to.len() {
                    for (from, to) in from.into_iter().zip(to) {
                        if project.glyphs.contains_key(&from) && project.glyphs.contains_key(&to) {
                            substitutions.push((from, to));
                        }
                    }
                }
                continue;
            }
            if by > sub + 2 && by + 1 < tokens.len() {
                ligatures.push((
                    tokens[sub + 1..by]
                        .iter()
                        .map(|name| (*name).to_string())
                        .collect::<Vec<_>>(),
                    tokens[by + 1]
                        .trim_matches(|character: char| "[]".contains(character))
                        .to_string(),
                ));
            } else if by == sub + 2 && by + 1 < tokens.len() {
                let from = tokens[sub + 1].trim_matches(|character: char| "[]".contains(character));
                let replacements = tokens[by + 1..]
                    .iter()
                    .map(|name| name.trim_matches(|character: char| "[]".contains(character)))
                    .filter(|name| project.glyphs.contains_key(*name))
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                if project.glyphs.contains_key(from) && replacements.len() == 1 {
                    substitutions.push((from.to_string(), replacements[0].clone()));
                } else if project.glyphs.contains_key(from) && replacements.len() > 1 {
                    multiples.push((from.to_string(), replacements));
                }
            }
        }
    }
    for (components, replacement) in ligatures {
        if components.len() < 2 || !project.glyphs.contains_key(&replacement) {
            continue;
        }
        let mut index = 0;
        while index + components.len() <= names.len() {
            if names[index..index + components.len()] == components[..] {
                names.splice(index..index + components.len(), [replacement.clone()]);
            } else {
                index += 1;
            }
        }
    }
    for (from, to) in substitutions {
        for name in &mut names {
            if *name == from {
                *name = to.clone();
            }
        }
    }
    for (from, to) in alternates {
        for name in &mut names {
            if *name == from {
                *name = to.clone();
            }
        }
    }
    for (sequence, target_index, replacement) in contexts {
        if sequence.len() > names.len() {
            continue;
        }
        let mut index = 0;
        while index + sequence.len() <= names.len() {
            if names[index..index + sequence.len()] == sequence[..] {
                names[index + target_index] = replacement.clone();
                index += sequence.len();
            } else {
                index += 1;
            }
        }
    }
    for (from, replacements) in multiples {
        let mut index = 0;
        while index < names.len() {
            if names[index] == from {
                names.splice(index..=index, replacements.clone());
                index += replacements.len();
            } else {
                index += 1;
            }
        }
    }
    names
}

pub(super) fn preview_feature_enabled(features: &str, tag: &str) -> bool {
    features
        .split([',', ' ', '\t'])
        .map(str::trim)
        .any(|candidate| candidate == tag)
}

pub(super) fn toggle_preview_feature(features: &mut String, tag: &str) {
    let mut tags: Vec<String> = features
        .split([',', ' ', '\t'])
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty() && *candidate != tag)
        .map(str::to_string)
        .collect();
    if !preview_feature_enabled(features, tag) {
        tags.push(tag.to_string());
    }
    *features = tags.join(",");
}

pub(super) fn preview_contour_points(contour: &Contour, origin: Pos2, scale: f32) -> Vec<Pos2> {
    let mut points = Vec::new();
    flatten(contour.to_bezpath(), 0.5, |element| {
        if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
            points.push(Pos2::new(
                origin.x + point.x as f32 * scale,
                origin.y - point.y as f32 * scale,
            ));
        }
    });
    points
}

pub(super) type PreviewTransform = (f64, f64, f64, f64, f64, f64);

pub(super) fn component_transform(component: &GlyphComponent) -> PreviewTransform {
    (
        component.x_scale,
        component.xy_scale,
        component.yx_scale,
        component.y_scale,
        component.x_offset,
        component.y_offset,
    )
}

pub(super) fn max_projected_outline_x(
    project: &FontProject,
    glyph_name: &str,
    transform: PreviewTransform,
    visiting: &mut std::collections::HashSet<String>,
) -> Option<f64> {
    if !visiting.insert(glyph_name.to_string()) {
        return None;
    }
    let mut max_x = None;
    if let Some(glyph) = project.glyphs.get(glyph_name) {
        let (a, b, _, _, e, _) = transform;
        for contour in &glyph.contours {
            flatten(contour.to_bezpath(), 0.25, |element| {
                if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
                    let x = a * point.x + b * point.y + e;
                    if x.is_finite() {
                        max_x = Some(max_x.map_or(x, |current: f64| current.max(x)));
                    }
                }
            });
        }
        for component in &glyph.components {
            let child_max = max_projected_outline_x(
                project,
                &component.base,
                compose_preview_transform(transform, component_transform(component)),
                visiting,
            );
            if let Some(x) = child_max {
                max_x = Some(max_x.map_or(x, |current: f64| current.max(x)));
            }
        }
    }
    visiting.remove(glyph_name);
    max_x
}

pub(super) fn min_projected_outline_x(
    project: &FontProject,
    glyph_name: &str,
    transform: PreviewTransform,
    visiting: &mut std::collections::HashSet<String>,
) -> Option<f64> {
    if !visiting.insert(glyph_name.to_string()) {
        return None;
    }
    let mut min_x = None;
    if let Some(glyph) = project.glyphs.get(glyph_name) {
        let (a, b, _, _, e, _) = transform;
        for contour in &glyph.contours {
            flatten(contour.to_bezpath(), 0.25, |element| {
                if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
                    let x = a * point.x + b * point.y + e;
                    if x.is_finite() {
                        min_x = Some(min_x.map_or(x, |current: f64| current.min(x)));
                    }
                }
            });
        }
        for component in &glyph.components {
            let child_min = min_projected_outline_x(
                project,
                &component.base,
                compose_preview_transform(transform, component_transform(component)),
                visiting,
            );
            if let Some(x) = child_min {
                min_x = Some(min_x.map_or(x, |current: f64| current.min(x)));
            }
        }
    }
    visiting.remove(glyph_name);
    min_x
}

pub(super) fn compose_preview_transform(
    parent: PreviewTransform,
    child: PreviewTransform,
) -> PreviewTransform {
    let (a, b, c, d, e, f) = parent;
    let (g, h, i, j, k, l) = child;
    (
        a * g + b * i,
        a * h + b * j,
        c * g + d * i,
        c * h + d * j,
        a * k + b * l + e,
        c * k + d * l + f,
    )
}

pub(super) fn preview_nested_component_polygons(
    project: &FontProject,
    glyph_name: &str,
    origin: Pos2,
    scale: f32,
    transform: PreviewTransform,
    visiting: &mut std::collections::HashSet<String>,
    polygons: &mut Vec<Vec<Pos2>>,
) {
    if !visiting.insert(glyph_name.to_string()) {
        return;
    }
    if let Some(glyph) = project.glyphs.get(glyph_name) {
        for contour in &glyph.contours {
            let mut points = Vec::new();
            flatten(contour.to_bezpath(), 0.5, |element| {
                if let PathEl::MoveTo(point) | PathEl::LineTo(point) = element {
                    let (a, b, c, d, e, f) = transform;
                    points.push(Pos2::new(
                        origin.x + (a * point.x + b * point.y + e) as f32 * scale,
                        origin.y - (c * point.x + d * point.y + f) as f32 * scale,
                    ));
                }
            });
            polygons.push(points);
        }
        for component in &glyph.components {
            preview_nested_component_polygons(
                project,
                &component.base,
                origin,
                scale,
                compose_preview_transform(transform, component_transform(component)),
                visiting,
                polygons,
            );
        }
    }
    visiting.remove(glyph_name);
}

pub(super) fn glyph_name_for_char(ch: char) -> String {
    format!("uni{:04X}", ch as u32)
}

pub(super) fn glyph_name_for_project_char(project: &FontProject, ch: char) -> String {
    let codepoint = ch as u32;
    project
        .glyphs
        .values()
        .find(|glyph| glyph.unicode == Some(codepoint) || glyph.unicodes.contains(&codepoint))
        .map(|glyph| glyph.name.clone())
        .unwrap_or_else(|| glyph_name_for_char(ch))
}

pub(super) fn master_compatibility_issues(
    project: &FontProject,
    from_master_id: &str,
    to_master_id: &str,
) -> Vec<String> {
    let mut issues = Vec::new();
    for glyph in project.glyphs.values() {
        let Some(from) = glyph.layers.get(from_master_id) else {
            continue;
        };
        let Some(to) = glyph.layers.get(to_master_id) else {
            continue;
        };
        if from.interpolate(to, 0.5).is_none() {
            let reason = if from.contours.len() != to.contours.len() {
                "輪郭数"
            } else if from
                .contours
                .iter()
                .zip(&to.contours)
                .any(|(a, b)| a.points.len() != b.points.len())
            {
                "ノード数"
            } else if from.components.len() != to.components.len() {
                "コンポーネント数"
            } else if from.anchors.len() != to.anchors.len() {
                "アンカー数"
            } else if from
                .components
                .iter()
                .zip(&to.components)
                .any(|(a, b)| a.base != b.base)
            {
                "コンポーネント名"
            } else if from
                .anchors
                .iter()
                .any(|a| !to.anchors.iter().any(|b| a.name == b.name))
            {
                "アンカー名"
            } else if from.contours.iter().zip(&to.contours).any(|(a, b)| {
                a.points
                    .iter()
                    .zip(&b.points)
                    .any(|(from_point, to_point)| from_point.point_type != to_point.point_type)
            }) {
                "ノード種別"
            } else {
                "構成"
            };
            issues.push(format!("{}: {}が不一致", glyph.name, reason));
        }
    }
    issues.sort();
    issues
}
