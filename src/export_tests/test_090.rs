    #[test]
    fn feature_source_mark_to_mark_compiles_from_mark_class() {
        let mut project = FontProject::new();
        project.add_glyph("acute".into(), None);
        project.add_glyph("grave".into(), None);
        let ids = [("acute", 1), ("grave", 2)].into_iter().collect();
        let source = "markClass acute <anchor 0 0> @TOP; markClass grave <anchor 10 0> @TOP; feature mkmk { pos mark @TOP mark @TOP; } mkmk;";
        assert!(build_kerning_gpos(&project, &ids, source).is_some());
    }
