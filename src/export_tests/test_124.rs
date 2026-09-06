    #[test]
    fn stat_table_encodes_named_multi_axis_values() {
        let table = build_stat_table_with_values(
            &[(*b"wght", 256), (*b"wdth", 257)],
            &[vec![700.0, 110.0]],
            &[300],
        );
        assert_eq!(u16::from_be_bytes([table[12], table[13]]), 1);
        assert_eq!(
            u32::from_be_bytes([table[14], table[15], table[16], table[17]]),
            36
        );
        assert_eq!(u16::from_be_bytes([table[36], table[37]]), 38);
        assert_eq!(u16::from_be_bytes([table[38], table[39]]), 4);
        assert_eq!(u16::from_be_bytes([table[40], table[41]]), 2);
        assert_eq!(u16::from_be_bytes([table[44], table[45]]), 300);
    }
