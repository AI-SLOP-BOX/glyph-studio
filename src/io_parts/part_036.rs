
fn import_cursive_anchors(
    face: &ttf_parser::Face<'_>,
    names: &[String],
    project: &mut FontProject,
) {
    let Some(gpos) = face.tables().gpos else {
        return;
    };
    for feature in gpos.features {
        for lookup_index in feature.lookup_indices {
            let Some(lookup) = gpos.lookups.get(lookup_index) else {
                continue;
            };
            for subtable in lookup
                .subtables
                .into_iter::<ttf_parser::gpos::PositioningSubtable>()
            {
                let ttf_parser::gpos::PositioningSubtable::Cursive(table) = subtable else {
                    continue;
                };
                for raw_id in 0..names.len() {
                    let glyph_id = ttf_parser::GlyphId(raw_id as u16);
                    let Some(index) = table.coverage.get(glyph_id) else {
                        continue;
                    };
                    let Some(name) = feature_glyph_name(names, glyph_id) else {
                        continue;
                    };
                    if let Some(anchor) = table.sets.entry(index) {
                        if anchor.x_device.is_none() && anchor.y_device.is_none() {
                            add_imported_anchor(project, name, "entry".into(), anchor.x, anchor.y);
                        }
                    }
                    if let Some(anchor) = table.sets.exit(index) {
                        if anchor.x_device.is_none() && anchor.y_device.is_none() {
                            add_imported_anchor(project, name, "exit".into(), anchor.x, anchor.y);
                        }
                    }
                }
            }
        }
    }
}
