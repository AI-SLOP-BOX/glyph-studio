    #[test]
    fn feature_source_validation_rejects_invalid_or_duplicate_tags() {
        assert!(validate_feature_source("feature lig { sub f i by fi; } lig;").is_err());
        assert!(validate_feature_source("feature ligä { sub f i by fi; } ligä;").is_err());
        assert!(validate_feature_source(
            "feature liga { sub f i by fi; } liga; feature liga { sub f by f.alt; } liga;"
        )
        .is_err());
    }
