    #[test]
    fn invalid_multibyte_feature_tag_does_not_panic() {
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("B", 2)]);
        let result = std::panic::catch_unwind(|| {
            build_simple_gsub("feature あいうえ { sub A by B; } あいうえ;", &glyph_ids)
        });
        assert!(result.is_ok());
    }
