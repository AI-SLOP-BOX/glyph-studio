
fn build_cmap_with_variations(
    mapping: &BTreeMap<u32, u16>,
    variations: &[UnicodeVariationSequence],
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Vec<u8> {
    let format4 = build_cmap_format4(mapping);
    let format12 = build_cmap_format12(mapping);
    let format12_subtable = &format12[12..];
    let format14 = build_cmap_format14(variations, glyph_ids);
    let subtable_count = 4 + 2 * usize::from(format14.is_some());
    let header_length = 4 + subtable_count * 8;
    let mut records = vec![
        (0_u16, 3_u16, format4.as_slice()),
        (3_u16, 1_u16, format4.as_slice()),
        (0_u16, 4_u16, format12_subtable),
        (3_u16, 10_u16, format12_subtable),
    ];
    if let Some(format14) = format14.as_ref() {
        records.push((0_u16, 5_u16, format14.as_slice()));
        records.push((3_u16, 10_u16, format14.as_slice()));
    }
    let mut output = Vec::with_capacity(
        header_length + records.iter().map(|(_, _, data)| data.len()).sum::<usize>(),
    );
    output.extend_from_slice(&0_u16.to_be_bytes());
    output.extend_from_slice(&(subtable_count as u16).to_be_bytes());
    let mut offset = header_length;
    for (platform, encoding, data) in &records {
        output.extend_from_slice(&platform.to_be_bytes());
        output.extend_from_slice(&encoding.to_be_bytes());
        output.extend_from_slice(&(offset as u32).to_be_bytes());
        offset += data.len();
    }
    for (_, _, data) in records {
        output.extend_from_slice(data);
    }
    output
}
