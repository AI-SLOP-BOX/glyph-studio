
#[cfg_attr(not(test), allow(dead_code))]
fn build_cmap_with_bmp_and_full_unicode(mapping: &BTreeMap<u32, u16>) -> Vec<u8> {
    let format4 = build_cmap_format4(mapping);
    let format12 = build_cmap_format12(mapping);
    let format12_subtable = &format12[12..];
    let records = [
        (0_u16, 3_u16, format4.as_slice()),
        (3_u16, 1_u16, format4.as_slice()),
        (0_u16, 4_u16, format12_subtable),
        (3_u16, 10_u16, format12_subtable),
    ];
    let header_length = 4 + records.len() * 8;
    let mut output =
        Vec::with_capacity(header_length + format4.len() * 2 + format12_subtable.len() * 2);
    output.extend_from_slice(&0_u16.to_be_bytes());
    output.extend_from_slice(&(records.len() as u16).to_be_bytes());
    let mut offset = header_length;
    for (platform, encoding, data) in records {
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
