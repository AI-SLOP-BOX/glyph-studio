
/// Returns the four OS/2 Unicode-range bitfields. The ranges are deliberately
/// block-oriented: a bit is advertised when at least one mapped code point is
/// in that Unicode block, which is the convention used by font consumers.
fn unicode_range_bits(mapping: &BTreeMap<u32, u16>) -> (u32, u32, u32, u32) {
    const RANGES: &[(u32, u32, u8)] = &[
        (0x0000, 0x007F, 0),  // Basic Latin
        (0x0080, 0x00FF, 1),  // Latin-1 Supplement
        (0x0100, 0x017F, 2),  // Latin Extended-A
        (0x0180, 0x024F, 3),  // Latin Extended-B
        (0x0250, 0x02AF, 4),  // IPA Extensions
        (0x02B0, 0x02FF, 5),  // Spacing Modifier Letters
        (0x0300, 0x036F, 6),  // Combining Diacritical Marks
        (0x0370, 0x03FF, 7),  // Greek and Coptic
        (0x0400, 0x04FF, 8),  // Cyrillic
        (0x0530, 0x058F, 9),  // Armenian
        (0x0590, 0x05FF, 10), // Hebrew
        (0x0600, 0x06FF, 11), // Arabic
        (0x0900, 0x097F, 12), // Devanagari
        (0x0980, 0x09FF, 13), // Bengali
        (0x0A00, 0x0A7F, 14), // Gurmukhi
        (0x0A80, 0x0AFF, 15), // Gujarati
        (0x0B00, 0x0B7F, 16), // Oriya
        (0x0B80, 0x0BFF, 17), // Tamil
        (0x0C00, 0x0C7F, 18), // Telugu
        (0x0C80, 0x0CFF, 19), // Kannada
        (0x0D00, 0x0D7F, 20), // Malayalam
        (0x0E00, 0x0E7F, 21), // Thai
        (0x0E80, 0x0EFF, 22), // Lao
        (0x10A0, 0x10FF, 23), // Georgian
        (0x1100, 0x11FF, 24), // Hangul Jamo
        (0x1E00, 0x1EFF, 25), // Latin Extended Additional
        (0x1F00, 0x1FFF, 26), // Greek Extended
        (0x2000, 0x206F, 27), // General Punctuation
        (0x2070, 0x209F, 28), // Superscripts and Subscripts
        (0x20A0, 0x20CF, 29), // Currency Symbols
        (0x20D0, 0x20FF, 30), // Combining Diacritical Marks for Symbols
        (0x2100, 0x214F, 31), // Letterlike Symbols
        (0x2150, 0x218F, 32), // Number Forms
        (0x2190, 0x21FF, 33), // Arrows
        (0x2200, 0x22FF, 34), // Mathematical Operators
        (0x2300, 0x23FF, 35), // Miscellaneous Technical
        (0x2500, 0x257F, 36), // Box Drawing
        (0x2580, 0x259F, 37), // Block Elements
        (0x25A0, 0x25FF, 38), // Geometric Shapes
        (0x2600, 0x26FF, 39), // Miscellaneous Symbols
        (0x2700, 0x27BF, 40), // Dingbats
        (0x3000, 0x303F, 48), // CJK Symbols and Punctuation
        (0x3040, 0x309F, 49), // Hiragana
        (0x30A0, 0x30FF, 50), // Katakana
        (0x3100, 0x312F, 51), // Bopomofo
        (0x3130, 0x318F, 52), // Hangul Compatibility Jamo
        (0x31A0, 0x31BF, 53), // Bopomofo Extended
        (0x31F0, 0x31FF, 54), // Katakana Phonetic Extensions
        (0x4E00, 0x9FFF, 59), // CJK Unified Ideographs
        (0xAC00, 0xD7AF, 56), // Hangul Syllables
        (0xF900, 0xFAFF, 60), // CJK Compatibility Ideographs
        (0xFE30, 0xFE4F, 61), // CJK Compatibility Forms
        (0xFF00, 0xFFEF, 62), // Halfwidth and Fullwidth Forms
    ];
    let mut bits = [0u32; 4];
    for &codepoint in mapping.keys() {
        for &(start, end, bit) in RANGES {
            if (start..=end).contains(&codepoint) && bit < 128 {
                bits[(bit / 32) as usize] |= 1u32 << (bit % 32);
            }
        }
    }
    (bits[0], bits[1], bits[2], bits[3])
}
