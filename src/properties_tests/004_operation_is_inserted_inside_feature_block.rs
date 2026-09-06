    #[test]
    fn operation_is_inserted_inside_feature_block() {
        let mut source = "feature liga {\n    sub f i by fi;\n} liga;\n".to_string();
        insert_feature_operation(&mut source, "liga", "    sub s t by st;\n");
        assert_eq!(
            source,
            "feature liga {\n    sub f i by fi;\n    sub s t by st;\n} liga;\n"
        );
    }
