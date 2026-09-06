
/// Export project outlines, Unicode mappings, and horizontal metrics as TrueType.
fn materialize_conditional_substitutions(
    project: &mut FontProject,
) -> (Vec<ConditionalSubstitution>, AxisBounds) {
    let base = project
        .masters
        .iter()
        .find(|master| master.id == project.default_master_id)
        .or_else(|| project.masters.first())
        .cloned()
        .unwrap_or_default();
    let default_master = project
        .masters
        .iter()
        .find(|master| master.id == project.default_master_id)
        .or_else(|| project.masters.first());
    let mut axis_tags: Vec<String> = project
        .masters
        .iter()
        .flat_map(|master| master.axes.keys())
        .filter(|tag| tag.len() == 4 && tag.is_ascii())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .filter(|tag| {
            let first = default_master
                .and_then(|master| master.axes.get(tag))
                .copied()
                .unwrap_or(0.0);
            project.masters.iter().any(|master| {
                (master.axes.get(tag).copied().unwrap_or(0.0) - first).abs() > f64::EPSILON
            })
        })
        .collect();
    let has_width_axis = project
        .masters
        .iter()
        .any(|master| (master.width - base.width).abs() > f64::EPSILON);
    if axis_tags.is_empty() {
        axis_tags.push("wght".into());
    }
    if has_width_axis && !axis_tags.iter().any(|tag| tag == "wdth") {
        axis_tags.push("wdth".into());
    }
    let mut axis_bounds = AxisBounds::new();
    for (index, tag) in axis_tags.into_iter().enumerate() {
        let values: Vec<f64> = project
            .masters
            .iter()
            .map(|master| match tag.as_str() {
                "wght" => master.axes.get(&tag).copied().unwrap_or(master.weight),
                "wdth" => master.axes.get(&tag).copied().unwrap_or(master.width),
                _ => master.axes.get(&tag).copied().unwrap_or(0.0),
            })
            .collect();
        let Some(default) = default_master
            .map(|master| match tag.as_str() {
                "wght" => master.axes.get(&tag).copied().unwrap_or(master.weight),
                "wdth" => master.axes.get(&tag).copied().unwrap_or(master.width),
                _ => master.axes.get(&tag).copied().unwrap_or(0.0),
            })
            .or_else(|| values.first().copied())
        else {
            continue;
        };
        let min = values.iter().copied().fold(default, f64::min);
        let max = values.iter().copied().fold(default, f64::max);
        axis_bounds.insert(tag, (index as u16, min, default, max));
    }
    let mut substitutions = Vec::new();
    for (base_name, layers) in project.conditional_layers.clone() {
        let Some(base_glyph) = project.glyphs.get(&base_name).cloned() else {
            continue;
        };
        for (index, conditional_layer) in layers.into_iter().enumerate() {
            let suffix: String = conditional_layer
                .id
                .chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() || character == '_' {
                        character
                    } else {
                        '_'
                    }
                })
                .collect();
            let name_stem = format!(".cond.{base_name}.{suffix}-{index}");
            let mut alternate_name = name_stem.clone();
            let mut collision = 1;
            while project.glyphs.contains_key(&alternate_name) {
                alternate_name = format!("{name_stem}-{collision}");
                collision += 1;
            }
            let mut alternate = base_glyph.clone();
            alternate.name = alternate_name.clone();
            alternate.unicode = None;
            alternate.unicodes.clear();
            alternate.width = conditional_layer.layer.width;
            alternate.contours = conditional_layer.layer.contours.clone();
            alternate.components = conditional_layer.layer.components.clone();
            alternate.anchors = conditional_layer.layer.anchors.clone();
            for master in &project.masters {
                alternate
                    .layers
                    .insert(master.id.clone(), conditional_layer.layer.clone());
            }
            project.glyphs.insert(alternate_name.clone(), alternate);
            substitutions.push(ConditionalSubstitution {
                base: base_name.clone(),
                alternate: alternate_name,
                conditions: conditional_layer.conditions,
            });
        }
    }
    substitutions.sort_by(|left, right| {
        let specificity = right.conditions.len().cmp(&left.conditions.len());
        if specificity != std::cmp::Ordering::Equal {
            return specificity;
        }
        let span = |substitution: &ConditionalSubstitution| {
            substitution.conditions.values().fold(0.0, |total, range| {
                total
                    + match (range.min, range.max) {
                        (Some(min), Some(max)) => (max - min).max(0.0),
                        _ => f64::INFINITY,
                    }
            })
        };
        span(left)
            .partial_cmp(&span(right))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    (substitutions, axis_bounds)
}
