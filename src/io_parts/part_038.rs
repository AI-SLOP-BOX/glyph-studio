
fn import_mark_to_ligature_anchors(
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
                let ttf_parser::gpos::PositioningSubtable::MarkToLigature(table) = subtable else {
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
                    let ligature_id = ttf_parser::GlyphId(raw_id as u16);
                    let Some(ligature_index) = table.ligature_coverage.get(ligature_id) else {
                        continue;
                    };
                    let Some(name) = feature_glyph_name(names, ligature_id) else {
                        continue;
                    };
                    let Some(components) = table.ligature_array.get(ligature_index) else {
                        continue;
                    };
                    for component in 0..components.rows {
                        for class in 0..components.cols {
                            let Some(anchor) = components.get(component, class) else {
                                continue;
                            };
                            if anchor.x_device.is_some() || anchor.y_device.is_some() {
                                continue;
                            }
                            add_imported_anchor(
                                project,
                                name,
                                format!("class{class}_{}", component + 1),
                                anchor.x,
                                anchor.y,
                            );
                        }
                    }
                }
            }
        }
    }
}
