
/// Imports MarkToBase anchors into the editable glyph model. Class numbers
/// are given stable names so the exporter can rebuild a valid mark class and
/// base attachment lookup on the next export.
fn import_mark_to_base_anchors(
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
                let ttf_parser::gpos::PositioningSubtable::MarkToBase(table) = subtable else {
                    continue;
                };
                for raw_id in 0..names.len() {
                    let mark_id = ttf_parser::GlyphId(raw_id as u16);
                    let Some(mark_index) = table.mark_coverage.get(mark_id) else {
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
                    let base_id = ttf_parser::GlyphId(raw_id as u16);
                    let Some(base_index) = table.base_coverage.get(base_id) else {
                        continue;
                    };
                    let Some(name) = feature_glyph_name(names, base_id) else {
                        continue;
                    };
                    for class in 0..table.anchors.cols {
                        let Some(anchor) = table.anchors.get(base_index, class) else {
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
