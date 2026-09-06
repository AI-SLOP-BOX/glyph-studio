
fn imported_single_substitution_map(
    lookup: &read_fonts::tables::gsub::SubstitutionLookup<'_>,
) -> std::collections::HashMap<u16, u16> {
    let mut substitutions = std::collections::HashMap::new();
    let Ok(SubstitutionSubtables::Single(subtables)) = lookup.subtables() else {
        return substitutions;
    };
    for subtable in subtables.iter().flatten() {
        match subtable {
            SingleSubst::Format1(table) => {
                let Ok(coverage) = table.coverage() else {
                    continue;
                };
                for glyph in coverage.iter() {
                    let source = glyph.to_u16();
                    let target = i32::from(source) + i32::from(table.delta_glyph_id());
                    if let Ok(target) = u16::try_from(target) {
                        substitutions.insert(source, target);
                    }
                }
            }
            SingleSubst::Format2(table) => {
                let Ok(coverage) = table.coverage() else {
                    continue;
                };
                for (index, glyph) in coverage.iter().enumerate() {
                    if let Some(target) = table.substitute_glyph_ids().get(index) {
                        substitutions.insert(glyph.to_u16(), target.get().to_u16());
                    }
                }
            }
        }
    }
    substitutions
}
