    #[test]
    fn cmap_format12_preserves_supplementary_codepoints() {
        let mapping = BTreeMap::from([(0x1F600, 4_u16), (0x1F601, 5_u16)]);
        let bytes = build_cmap_format12(&mapping);
        assert_eq!(u16::from_be_bytes([bytes[4], bytes[5]]), 3);
        assert_eq!(u16::from_be_bytes([bytes[6], bytes[7]]), 10);
        assert_eq!(u16::from_be_bytes([bytes[0], bytes[1]]), 0);
        assert_eq!(u16::from_be_bytes([bytes[2], bytes[3]]), 1);
        assert_eq!(u16::from_be_bytes([bytes[12], bytes[13]]), 12);
        assert_eq!(
            u32::from_be_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]),
            0x1F600
        );
    }
