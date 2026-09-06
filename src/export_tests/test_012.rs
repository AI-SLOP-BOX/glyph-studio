    #[test]
    fn named_lookup_references_are_expanded_into_gpos() {
        let project = FontProject::new();
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("V", 2)]);
        let source = "lookup pairAdjust { pos A V <0 0 -80 0>; } pairAdjust;\nfeature kern { lookup pairAdjust; } kern;";
        let bytes = build_kerning_gpos(&project, &glyph_ids, source)
            .expect("named GPOS lookup should compile");
        assert!(bytes.len() > 40);
    }
