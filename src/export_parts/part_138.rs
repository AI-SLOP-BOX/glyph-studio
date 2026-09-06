
fn code_page_range_bits(mapping: &BTreeMap<u32, u16>) -> (u32, u32) {
    let mut range1 = 0u32;
    let mut range2 = 0u32;
    let has = |ranges: &[(u32, u32)]| {
        mapping.keys().any(|codepoint| {
            ranges
                .iter()
                .any(|(start, end)| (*start..=*end).contains(codepoint))
        })
    };
    for (bit, ranges) in [
        (0, &[(0x0000, 0x007F), (0x00A0, 0x00FF)][..]), // Latin 1 / 1252
        (1, &[(0x0100, 0x024F), (0x1E00, 0x1EFF)][..]), // Latin 2 / 1250
        (2, &[(0x0400, 0x04FF)][..]),                   // Cyrillic / 1251
        (3, &[(0x0370, 0x03FF)][..]),                   // Greek / 1253
        (4, &[(0x0100, 0x017F)][..]),                   // Turkish / 1254
        (5, &[(0x0590, 0x05FF)][..]),                   // Hebrew / 1255
        (6, &[(0x0600, 0x06FF)][..]),                   // Arabic / 1256
        (16, &[(0x0E00, 0x0E7F)][..]),                  // Thai / 874
        (17, &[(0x3040, 0x30FF), (0x4E00, 0x9FFF)][..]), // Japanese / 932
        (19, &[(0xAC00, 0xD7AF)][..]),                  // Korean / 949
        (20, &[(0xFF00, 0xFFEF)][..]),                  // Traditional CJK / 950
    ] {
        if has(ranges) {
            if bit < 32 {
                range1 |= 1u32 << bit;
            } else {
                range2 |= 1u32 << (bit - 32);
            }
        }
    }
    (range1, range2)
}
