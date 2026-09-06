    #[test]
    fn feature_table_name_records_parse_custom_and_localized_names() {
        let records = parse_feature_name_records(
            "table name { nameid 256 \"Display Name\"; nameid 257 3 1 0x411 \"表示名\"; } name;",
        );
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].nameID, 256);
        assert_eq!(records[0].string, "Display Name");
        assert_eq!(records[1].platformID, 3);
        assert_eq!(records[1].encodingID, 1);
        assert_eq!(records[1].languageID, 0x411);
    }
