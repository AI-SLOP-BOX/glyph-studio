    #[test]
    fn feature_source_validation_checks_languagesystem_tags() {
        assert!(validate_feature_source(
            "languagesystem latn dflt; feature liga { sub f i by fi; } liga;"
        )
        .is_ok());
        assert!(validate_feature_source(
            "languagesystem latin dflt; feature liga { sub f i by fi; } liga;"
        )
        .is_err());
        assert!(validate_feature_source(
            "languagesystem latn Japanese; feature liga { sub f i by fi; } liga;"
        )
        .is_err());
        assert!(validate_feature_source(
            "# languagesystem bad dflt;\nfeature liga { sub f i by fi; } liga;"
        )
        .is_ok());
    }
