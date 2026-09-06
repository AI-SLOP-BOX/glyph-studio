    #[test]
    fn simple_gsub_supports_alternate_substitution() {
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("A.alt", 2), ("A.swash", 3)]);
        let bytes = build_simple_gsub(
            "feature salt { sub A from [A.alt A.swash]; } salt;",
            &glyph_ids,
        );
        assert!(bytes.is_some());
        let bytes = bytes.unwrap();
        assert!(bytes.len() > 20);
        assert!(bytes.windows(4).any(|window| window == b"salt"));
    }
