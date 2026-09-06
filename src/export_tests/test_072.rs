    #[test]
    fn feature_source_cursive_allows_null_anchors() {
        let project = FontProject::new();
        let ids = [("alef", 1), ("beh", 2)].into_iter().collect();
        let source = "feature curs { pos cursive alef NULL <anchor 500 500>; pos cursive beh <anchor 0 500> NULL; } curs;";
        let bytes = build_kerning_gpos(&project, &ids, source).unwrap();
        assert!(bytes.len() > 40);
    }
