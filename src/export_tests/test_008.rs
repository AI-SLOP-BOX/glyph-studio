    #[test]
    fn feature_source_accepts_enumerated_substitution_and_positioning() {
        let glyph_ids = std::collections::HashMap::from([("A", 1), ("A.alt", 2), ("V", 3)]);
        let gsub = build_simple_gsub("feature salt { enum sub A by A.alt; } salt;", &glyph_ids)
            .expect("enum sub should compile as an enumerated substitution");
        assert!(gsub.windows(4).any(|window| window == b"salt"));
        let enumerate = build_simple_gsub(
            "feature salt { enumerate sub A by A.alt; } salt;",
            &glyph_ids,
        )
        .expect("enumerate sub should compile as an enumerated substitution");
        assert!(enumerate.windows(4).any(|window| window == b"salt"));

        let project = FontProject::new();
        let gpos = build_kerning_gpos(
            &project,
            &glyph_ids,
            "feature kern { enum pos A V <0 0 -80 0>; } kern;",
        )
        .expect("enum pos should compile as an enumerated positioning rule");
        assert!(gpos.windows(4).any(|window| window == b"kern"));
    }
