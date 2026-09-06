    #[test]
    fn feature_source_mark_class_and_anchor_positioning_compile() {
        let mut project = FontProject::new();
        project.add_glyph("acute".into(), None);
        project.add_glyph("A".into(), None);
        let ids = [("acute", 1), ("A", 2)].into_iter().collect();
        let source = "markClass acute <anchor 0 0> @TOP; feature mark { pos base A <anchor 300 700> mark @TOP; } mark;";
        assert_eq!(
            parse_feature_anchor(&["markClass", "acute", "<anchor", "0", "0>", "@TOP"], 2),
            Some((0, 0))
        );
        assert!(build_kerning_gpos(&project, &ids, source).is_some());
    }
