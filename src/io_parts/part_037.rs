
fn import_mark_to_mark_anchors(
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
                let ttf_parser::gpos::PositioningSubtable::MarkToMark(table) = subtable else {
                    continue;
                };
                for raw_id in 0..names.len() {
                    let mark_id = ttf_parser::GlyphId(raw_id as u16);
                    let Some(mark_index) = table.mark1_coverage.get(mark_id) else {
                        continue;
                    };
                    let Some((class, anchor)) = table.marks.get(mark_index) else {
                        continue;
                    };
                    if anchor.x_device.is_some() || anchor.y_device.is_some() {
                        continue;
                    }
                    let Some(name) = feature_glyph_name(names, mark_id) else {
                        continue;
                    };
                    add_imported_anchor(
                        project,
                        name,
                        format!("_class{class}"),
                        anchor.x,
                        anchor.y,
                    );
                }
                for raw_id in 0..names.len() {
                    let mark2_id = ttf_parser::GlyphId(raw_id as u16);
                    let Some(mark2_index) = table.mark2_coverage.get(mark2_id) else {
                        continue;
                    };
                    let Some(name) = feature_glyph_name(names, mark2_id) else {
                        continue;
                    };
                    for class in 0..table.mark2_matrix.cols {
                        let Some(anchor) = table.mark2_matrix.get(mark2_index, class) else {
                            continue;
                        };
                        if anchor.x_device.is_some() || anchor.y_device.is_some() {
                            continue;
                        }
                        add_imported_anchor(
                            project,
                            name,
                            format!("class{class}"),
                            anchor.x,
                            anchor.y,
                        );
                    }
                }
            }
        }
    }
}
