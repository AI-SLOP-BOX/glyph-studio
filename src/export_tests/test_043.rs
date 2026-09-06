    #[test]
    fn os2_unicode_ranges_follow_mapped_blocks() {
        let mapping = BTreeMap::from([(65, 1_u16), (0x3042, 2_u16), (0x1F600, 3_u16)]);
        let (range1, range2, range3, range4) = unicode_range_bits(&mapping);
        assert_ne!(range1 & (1 << 0), 0); // Basic Latin
        assert_ne!(range2 & (1 << (49 - 32)), 0); // Hiragana
        assert_eq!(range3 | range4, 0);
        let (code_pages1, _) = code_page_range_bits(&mapping);
        assert_ne!(code_pages1 & (1 << 0), 0); // Windows Latin 1
        assert_ne!(code_pages1 & (1 << 17), 0); // Japanese
    }
