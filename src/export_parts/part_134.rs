
/// Fingerprint the project inputs that can change generated GDEF/GPOS or
/// invalidate glyph IDs. Outline coordinates are intentionally excluded: a
/// contour edit does not alter the layout tables.
pub(crate) fn layout_input_fingerprint(project: &FontProject) -> u64 {
    use std::hash::{Hash, Hasher};

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    project.glyph_names_sorted().hash(&mut hasher);
    for name in project.glyph_names_sorted() {
        let Some(glyph) = project.glyphs.get(name) else {
            continue;
        };
        glyph.unicode.hash(&mut hasher);
        glyph.unicodes.hash(&mut hasher);
        glyph.width.to_bits().hash(&mut hasher);
        glyph.left_kerning_group.hash(&mut hasher);
        glyph.right_kerning_group.hash(&mut hasher);
        glyph.anchors.len().hash(&mut hasher);
        for anchor in &glyph.anchors {
            anchor.name.hash(&mut hasher);
            anchor.x.to_bits().hash(&mut hasher);
            anchor.y.to_bits().hash(&mut hasher);
        }
    }
    let mut kerning = project.kerning.iter().collect::<Vec<_>>();
    kerning.sort_by(|left, right| left.0.cmp(right.0));
    for ((left, right), value) in kerning {
        left.hash(&mut hasher);
        right.hash(&mut hasher);
        value.to_bits().hash(&mut hasher);
    }
    hasher.finish()
}
