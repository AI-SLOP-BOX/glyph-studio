
fn imported_single_positioning_map(
    lookup: &read_fonts::tables::gpos::PositionLookup<'_>,
) -> std::collections::HashMap<u16, [i16; 4]> {
    let mut positions = std::collections::HashMap::new();
    let Ok(PositionSubtables::Single(subtables)) = lookup.subtables() else {
        return positions;
    };
    let value = |record: &ValueRecord| {
        Some([
            record.x_placement().unwrap_or(0),
            record.y_placement().unwrap_or(0),
            record.x_advance().unwrap_or(0),
            record.y_advance().unwrap_or(0),
        ])
    };
    for subtable in subtables.iter().flatten() {
        match subtable {
            SinglePos::Format1(table) => {
                let Ok(coverage) = table.coverage() else {
                    continue;
                };
                let Some(value) = value(&table.value_record()) else {
                    continue;
                };
                for glyph in coverage.iter() {
                    positions.insert(glyph.to_u16(), value);
                }
            }
            SinglePos::Format2(table) => {
                let Ok(coverage) = table.coverage() else {
                    continue;
                };
                for (index, glyph) in coverage.iter().enumerate() {
                    let Ok(record) = table.value_records().get(index) else {
                        continue;
                    };
                    if let Some(value) = value(&record) {
                        positions.insert(glyph.to_u16(), value);
                    }
                }
            }
        }
    }
    positions
}
