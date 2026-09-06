    #[test]
    fn named_lookup_references_are_expanded_into_the_feature() {
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("A.alt", 2)]);
        let source = "lookup stylisticA { sub A by A.alt; } stylisticA;\nfeature salt { lookup stylisticA; } salt;";
        let expanded = expand_named_feature_lookups(source);
        assert!(expanded.contains("sub A by A.alt;"));
        let bytes = build_simple_gsub(source, &glyph_ids).expect("named lookup should compile");
        assert!(bytes.windows(4).any(|window| window == b"salt"));
    }
