#[allow(clippy::type_complexity)]
fn collect_kerning_pairs<'a>(
    project: &'a FontProject,
    glyph_ids: &std::collections::HashMap<&'a str, u16>,
) -> (
    std::collections::BTreeMap<GlyphId16, Vec<(GlyphId16, i16, bool)>>,
    std::collections::BTreeMap<(String, String), i16>,
    std::collections::HashMap<&'a str, Vec<&'a str>>,
    std::collections::HashMap<&'a str, Vec<&'a str>>,
) {
    let mut grouped = std::collections::BTreeMap::<GlyphId16, Vec<(GlyphId16, i16, bool)>>::new();
    let mut class_pairs = std::collections::BTreeMap::<(String, String), i16>::new();
    let mut left_groups = std::collections::HashMap::<&str, Vec<&str>>::new();
    let mut right_groups = std::collections::HashMap::<&str, Vec<&str>>::new();
    for (name, glyph) in &project.glyphs {
        if !glyph.left_kerning_group.trim().is_empty() {
            left_groups.entry(glyph.left_kerning_group.trim()).or_default().push(name.as_str());
        }
        if !glyph.right_kerning_group.trim().is_empty() {
            right_groups.entry(glyph.right_kerning_group.trim()).or_default().push(name.as_str());
        }
    }
    let mut kerning_entries: Vec<_> = project.kerning.iter().collect();
    kerning_entries.sort_by(|((left_a, right_a), _), ((left_b, right_b), _)| (left_a, right_a).cmp(&(left_b, right_b)));
    // Collect canonical group values first; differing glyph-level values
    // remain explicit exceptions in the PairPos format-1 lookup.
    for ((left, right), value) in &kerning_entries {
        let Ok(value) = checked_i16(**value, "GPOSカーニング値") else {
            continue;
        };
        let left_group = project.glyphs.get(left).map(|g| g.left_kerning_group.trim()).filter(|g| !g.is_empty());
        let right_group = project.glyphs.get(right).map(|g| g.right_kerning_group.trim()).filter(|g| !g.is_empty());
        if let (Some(left_group), Some(right_group)) = (left_group, right_group) {
            class_pairs.entry((left_group.to_string(), right_group.to_string())).or_insert(value);
        }
    }
    for ((left, right), value) in &kerning_entries {
        let Ok(value) = checked_i16(**value, "GPOSカーニング値") else {
            continue;
        };
        let left_group = project.glyphs.get(left).map(|glyph| glyph.left_kerning_group.trim()).filter(|group| !group.is_empty());
        let right_group = project.glyphs.get(right).map(|glyph| glyph.right_kerning_group.trim()).filter(|group| !group.is_empty());
        if let (Some(left_group), Some(right_group)) = (left_group, right_group) {
            let pair = (left_group.to_string(), right_group.to_string());
            if class_pairs.get(&pair) == Some(&value) {
                continue;
            }
        }
        let left_names = project
            .glyphs
            .get(left)
            .and_then(|glyph| left_groups.get(glyph.left_kerning_group.trim()))
            .filter(|names| !names.is_empty())
            .cloned()
            .unwrap_or_else(|| vec![left.as_str()]);
        let right_names = project
            .glyphs
            .get(right)
            .and_then(|glyph| right_groups.get(glyph.right_kerning_group.trim()))
            .filter(|names| !names.is_empty())
            .cloned()
            .unwrap_or_else(|| vec![right.as_str()]);
        for expanded_left in left_names {
            let Some(&left_id) = glyph_ids.get(expanded_left) else {
                continue;
            };
            for expanded_right in &right_names {
                let Some(&right_id) = glyph_ids.get(*expanded_right) else {
                    continue;
                };
                grouped
                    .entry(GlyphId16::new(left_id))
                    .or_default()
                    .push((GlyphId16::new(right_id), value, expanded_left == left.as_str() && *expanded_right == right.as_str()));
            }
        }
    }
    (grouped, class_pairs, left_groups, right_groups)
}
