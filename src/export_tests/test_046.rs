    #[test]
    fn cmap_format14_contains_unicode_variation_sequence_mappings() {
        let mapping = BTreeMap::from([(0x4E00, 1_u16)]);
        let variations = vec![UnicodeVariationSequence {
            base: 0x4E00,
            selector: 0xE0100,
            glyph: "A.ivs".into(),
        }];
        let glyph_ids = HashMap::from([("A.ivs", 2_u16)]);
        let bytes = build_cmap_with_variations(&mapping, &variations, &glyph_ids);
        assert_eq!(u16::from_be_bytes([bytes[2], bytes[3]]), 6);
        let format14_offset =
            u32::from_be_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]) as usize;
        assert_eq!(
            u16::from_be_bytes([bytes[format14_offset], bytes[format14_offset + 1]]),
            14
        );
        assert_eq!(
            u32::from_be_bytes([
                0,
                bytes[format14_offset + 10],
                bytes[format14_offset + 11],
                bytes[format14_offset + 12],
            ]),
            0xE0100
        );
    }
