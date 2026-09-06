    #[test]
    fn feature_source_compiles_ignore_positioning_rules() {
        let project = FontProject::new();
        let ids = [("A", 1), ("V", 2)].into_iter().collect();
        let bytes = build_kerning_gpos(
            &project,
            &ids,
            "feature kern { ignore pos A V; pos A V <0 0 -80 0>; } kern;",
        )
        .expect("ignore positioning should compile");
        assert!(!bytes.is_empty());
    }
