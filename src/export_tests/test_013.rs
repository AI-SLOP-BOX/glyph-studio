    #[test]
    fn feature_references_share_gpos_lookups() {
        let project = FontProject::new();
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("V", 2)]);
        let source = "feature krn2 { pos A V -80; } krn2; feature kern { feature krn2; } kern;";
        let bytes = build_kerning_gpos(&project, &glyph_ids, source)
            .expect("GPOS feature reference should compile");
        assert!(bytes.windows(4).any(|window| window == b"krn2"));
        assert!(bytes.windows(4).any(|window| window == b"kern"));
    }
