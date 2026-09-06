    #[test]
    fn feature_source_cursive_anchors_compile_into_gpos() {
        let project = FontProject::new();
        let ids = [("alef", 1), ("beh", 2)].into_iter().collect();
        let source = "feature curs { pos cursive alef <anchor 0 500> <anchor 500 500>; pos cursive beh <anchor 0 500> <anchor 500 500>; } curs;";
        let bytes = build_kerning_gpos(&project, &ids, source).unwrap();
        assert!(bytes.len() > 40);
    }
