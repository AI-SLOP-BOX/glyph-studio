    #[test]
    fn stat_table_has_a_valid_axis_directory_header() {
        let table = build_stat_table_with_values(&[(*b"wght", 256), (*b"wdth", 257)], &[], &[]);
        assert_eq!(&table[0..4], &0x0001_0002_u32.to_be_bytes());
        assert_eq!(u16::from_be_bytes([table[4], table[5]]), 8);
        assert_eq!(u16::from_be_bytes([table[6], table[7]]), 2);
        assert_eq!(
            u32::from_be_bytes([table[8], table[9], table[10], table[11]]),
            20
        );
        assert_eq!(&table[20..24], b"wght");
        assert_eq!(&table[28..32], b"wdth");
    }
