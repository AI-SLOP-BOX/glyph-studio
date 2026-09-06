    #[test]
    fn feature_source_validation_rejects_unbalanced_or_malformed_source() {
        assert!(validate_feature_source("feature liga { sub f i by fi; } liga;").is_ok());
        assert!(validate_feature_source("feature liga { sub f i by fi;").is_err());
        assert!(validate_feature_source("feature liga { sub f i by fi; };").is_ok());
        assert!(validate_feature_source("feature liga { sub f i by fi; }").is_ok());
    }
