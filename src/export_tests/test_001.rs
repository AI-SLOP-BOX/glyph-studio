    #[test]
    fn feature_names_override_default_stylistic_set_label() {
        let source = r#"
            feature ss01 {
                featureNames {
                    name "Handwritten Alternates";
                    name 3 1 0x409 "Localized Alternates";
                };
                sub A by A.alt;
            } ss01;
        "#;
        let records = feature_name_records(source, "ss01", 500);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].nameID, 500);
        assert_eq!(records[0].string, "Handwritten Alternates");
        assert_eq!(records[1].platformID, 3);
        assert_eq!(records[1].languageID, 0x409);
        assert_eq!(records[1].string, "Localized Alternates");
    }
