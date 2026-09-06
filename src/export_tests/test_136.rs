    #[test]
    fn feature_source_validation_checks_named_lookup_references() {
        let valid = "lookup L { sub f i by fi; } L; feature liga { lookup L; } liga;";
        assert!(validate_feature_source(valid).is_ok());
        assert!(validate_feature_source("feature liga { lookup Missing; } liga;").is_err());
        assert!(validate_feature_source(
            "lookup L { sub f i by fi; } L; lookup L { sub f by f.alt; } L;"
        )
        .is_err());
    }
