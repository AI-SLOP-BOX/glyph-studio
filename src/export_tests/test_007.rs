    #[test]
    fn use_extension_wraps_gsub_and_gpos_lookups() {
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("A.alt", 2), ("V", 3)]);
        let gsub = build_simple_gsub(
            "feature salt { lookupflag useExtension; sub A by A.alt; } salt;",
            &glyph_ids,
        )
        .expect("useExtension GSUB should compile");
        assert!(gsub.windows(2).any(|window| window == [0, 7]));

        let project = FontProject::new();
        let gpos = build_kerning_gpos(
            &project,
            &glyph_ids,
            "feature kern { useExtension; pos A V -80; } kern;",
        )
        .expect("useExtension GPOS should compile");
        assert!(gpos.windows(2).any(|window| window == [0, 9]));
    }
