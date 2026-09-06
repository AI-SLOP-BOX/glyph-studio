
fn build_cmap_format4(mapping: &BTreeMap<u32, u16>) -> Vec<u8> {
    let bmp = mapping
        .iter()
        .filter_map(|(&codepoint, &glyph_id)| {
            (codepoint <= 0xFFFF).then_some((codepoint as u16, glyph_id))
        })
        .collect::<Vec<_>>();
    let segment_count = bmp.len() + 1;
    let search_power = 1_u16 << (15 - (segment_count as u16).leading_zeros());
    let search_range = search_power * 2;
    let entry_selector = (15 - search_power.leading_zeros()) as u16;
    let range_shift = (segment_count as u16) * 2 - search_range;
    let length = 16 + segment_count * 8 + bmp.len() * 2;
    let mut output = Vec::with_capacity(length);
    output.extend_from_slice(&4_u16.to_be_bytes());
    output.extend_from_slice(&(length as u16).to_be_bytes());
    output.extend_from_slice(&0_u16.to_be_bytes());
    output.extend_from_slice(&((segment_count * 2) as u16).to_be_bytes());
    output.extend_from_slice(&search_range.to_be_bytes());
    output.extend_from_slice(&entry_selector.to_be_bytes());
    output.extend_from_slice(&range_shift.to_be_bytes());
    for &(codepoint, _) in &bmp {
        output.extend_from_slice(&codepoint.to_be_bytes());
    }
    output.extend_from_slice(&0xFFFF_u16.to_be_bytes());
    output.extend_from_slice(&0_u16.to_be_bytes());
    for &(codepoint, _) in &bmp {
        output.extend_from_slice(&codepoint.to_be_bytes());
    }
    output.extend_from_slice(&0xFFFF_u16.to_be_bytes());
    for _ in &bmp {
        output.extend_from_slice(&0_u16.to_be_bytes());
    }
    output.extend_from_slice(&1_u16.to_be_bytes());
    let glyph_array_start = 16 + segment_count * 8;
    for (index, _) in bmp.iter().enumerate() {
        let id_range_word =
            14 + segment_count * 2 + 2 + segment_count * 2 + segment_count * 2 + index * 2;
        let offset = glyph_array_start + index * 2 - id_range_word;
        output.extend_from_slice(&(offset as u16).to_be_bytes());
    }
    output.extend_from_slice(&0_u16.to_be_bytes());
    for (_, glyph_id) in bmp {
        output.extend_from_slice(&glyph_id.to_be_bytes());
    }
    output
}
