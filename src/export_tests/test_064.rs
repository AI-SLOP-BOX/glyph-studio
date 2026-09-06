    #[test]
    fn feature_source_accepts_short_pair_positioning_syntax() {
        let project = FontProject::new();
        let ids = [("A", 1), ("V", 2)].into_iter().collect();
        let short = build_kerning_gpos(&project, &ids, "feature kern { pos A V -80; } kern;")
            .expect("short pair positioning should compile");
        let long = build_kerning_gpos(
            &project,
            &ids,
            "feature kern { pos A V <0 0 -80 0>; } kern;",
        )
        .expect("ValueRecord pair positioning should compile");
        assert_eq!(short, long);
    }
