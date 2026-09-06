    #[test]
    fn mark_filtering_set_is_emitted_from_named_class() {
        let ids = [("acute", 1), ("grave", 2)].into_iter().collect();
        let source = "@Marks = [acute grave]; feature mark { lookupflag UseMarkFilteringSet @Marks; pos acute <0 0 0 0>; } mark;";
        let sets = parse_mark_glyph_sets(source, &ids);
        assert_eq!(sets.get("@Marks").map(|(index, _)| *index), Some(0));
        let mut project = FontProject::new();
        project.add_glyph("acute".into(), None);
        project.add_glyph("grave".into(), None);
        let bytes = build_gdef(&project, &ids, source);
        assert!(bytes.is_some());
    }
