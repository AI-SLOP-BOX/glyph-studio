    #[test]
    fn feature_validation_ignores_comments_and_accepts_multiline_declarations() {
        let source = "feature liga {\n  # } ; this is a comment\n  sub f i by fi;\n} liga;";
        assert!(validate_feature_source(source).is_ok());
        assert!(validate_feature_source("feature liga {\n  sub f i by fi;\n").is_err());
        assert!(validate_feature_source(
            "feature liga {\n  sub f i by fi;\n} liga;\n\"unterminated"
        )
        .is_err());
    }
