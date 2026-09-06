
fn build_cmap_format12(mapping: &BTreeMap<u32, u16>) -> Vec<u8> {
    let mut groups = Vec::<(u32, u32, u32)>::new();
    for (&codepoint, &glyph_id) in mapping {
        let can_extend = groups.last().map(|(start, end, start_glyph)| {
            codepoint == *end + 1
                && u64::from(*start_glyph) + u64::from(codepoint - *start) == u64::from(glyph_id)
        }) == Some(true);
        if can_extend {
            groups.last_mut().unwrap().1 = codepoint;
        } else {
            groups.push((codepoint, codepoint, u32::from(glyph_id)));
        }
    }
    let subtable_length = 16 + groups.len() as u32 * 12;
    let mut subtable = Vec::with_capacity(subtable_length as usize);
    subtable.extend_from_slice(&12_u16.to_be_bytes());
    subtable.extend_from_slice(&0_u16.to_be_bytes());
    subtable.extend_from_slice(&subtable_length.to_be_bytes());
    subtable.extend_from_slice(&0_u32.to_be_bytes());
    subtable.extend_from_slice(&(groups.len() as u32).to_be_bytes());
    for (start, end, start_glyph) in groups {
        subtable.extend_from_slice(&start.to_be_bytes());
        subtable.extend_from_slice(&end.to_be_bytes());
        subtable.extend_from_slice(&start_glyph.to_be_bytes());
    }
    let mut output = Vec::with_capacity(12 + subtable.len());
    output.extend_from_slice(&0_u16.to_be_bytes());
    output.extend_from_slice(&1_u16.to_be_bytes());
    output.extend_from_slice(&3_u16.to_be_bytes());
    output.extend_from_slice(&10_u16.to_be_bytes());
    output.extend_from_slice(&12_u32.to_be_bytes());
    output.extend_from_slice(&subtable);
    output
}
