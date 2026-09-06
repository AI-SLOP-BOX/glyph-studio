    #[test]
    fn tag_matching_does_not_use_a_similar_feature_name() {
        let mut source = "feature ligature {\n    sub f i by fi;\n} ligature;\n".to_string();
        insert_feature_operation(&mut source, "liga", "    sub s t by st;\n");
        assert!(source.contains("feature liga {\n    sub s t by st;\n}"));
        assert!(source.contains("feature ligature {\n    sub f i by fi;"));
    }
