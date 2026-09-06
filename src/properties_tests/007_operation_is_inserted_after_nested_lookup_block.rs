    #[test]
    fn operation_is_inserted_after_nested_lookup_block() {
        let mut source =
            "feature liga {\n    lookup L {\n        sub f i by fi;\n    } L;\n} liga;\n"
                .to_string();
        insert_feature_operation(&mut source, "liga", "    sub s t by st;\n");
        assert!(source.contains("} L;\n    sub s t by st;\n} liga;"));
    }
