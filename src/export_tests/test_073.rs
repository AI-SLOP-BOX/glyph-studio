    #[test]
    fn feature_value_records_parse_device_tables() {
        let records = parse_feature_value_records(
            "<-80 0 -160 0 <device 11 -1, 12 -1> <device NULL> <device 11 -2, 12 -2> <device NULL>>",
        );
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].values, [-80, 0, -160, 0]);
        assert!(records[0].devices[0].is_some());
        assert!(records[0].devices[1].is_none());
        assert!(records[0].devices[2].is_some());
        assert!(records[0].devices[3].is_none());
    }
