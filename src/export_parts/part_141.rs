
fn build_cmap_format14(
    variations: &[UnicodeVariationSequence],
    glyph_ids: &std::collections::HashMap<&str, u16>,
) -> Option<Vec<u8>> {
    let mut by_selector = BTreeMap::<u32, BTreeMap<u32, u16>>::new();
    for variation in variations {
        if variation.base > 0x10FFFF || variation.selector > 0xFFFFFF {
            continue;
        }
        let Some(&glyph_id) = glyph_ids.get(variation.glyph.as_str()) else {
            continue;
        };
        by_selector
            .entry(variation.selector)
            .or_default()
            .insert(variation.base, glyph_id);
    }
    if by_selector.is_empty() {
        return None;
    }
    let records_length = by_selector.len() * 11;
    let header_length = 10 + records_length;
    let mut records = Vec::with_capacity(records_length);
    let mut payload = Vec::new();
    for (selector, mappings) in by_selector {
        let offset = header_length + payload.len();
        records.extend_from_slice(&selector.to_be_bytes()[1..]);
        records.extend_from_slice(&0_u32.to_be_bytes());
        records.extend_from_slice(&(offset as u32).to_be_bytes());
        payload.extend_from_slice(&(mappings.len() as u32).to_be_bytes());
        for (base, glyph_id) in mappings {
            payload.extend_from_slice(&base.to_be_bytes()[1..]);
            payload.extend_from_slice(&glyph_id.to_be_bytes());
        }
    }
    let length = header_length + payload.len();
    let mut output = Vec::with_capacity(length);
    output.extend_from_slice(&14_u16.to_be_bytes());
    output.extend_from_slice(&(length as u32).to_be_bytes());
    output.extend_from_slice(&(by_selector_count(&records) as u32).to_be_bytes());
    output.extend_from_slice(&records);
    output.extend_from_slice(&payload);
    Some(output)
}
