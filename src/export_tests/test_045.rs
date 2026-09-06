    #[test]
    fn combined_cmap_keeps_bmp_and_supplementary_subtables() {
        let mapping = BTreeMap::from([(65, 2_u16), (0x1F600, 4_u16)]);
        let bytes = build_cmap_with_bmp_and_full_unicode(&mapping);
        assert_eq!(u16::from_be_bytes([bytes[2], bytes[3]]), 4);
        assert_eq!(u16::from_be_bytes([bytes[4], bytes[5]]), 0);
        let format4_offset =
            u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        let format12_offset =
            u32::from_be_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]) as usize;
        assert_eq!(
            u16::from_be_bytes([bytes[format4_offset], bytes[format4_offset + 1]]),
            4
        );
        assert_eq!(
            u16::from_be_bytes([bytes[format12_offset], bytes[format12_offset + 1]]),
            12
        );
    }
