    #[test]
    fn feature_source_expands_named_anchor_definitions() {
        let project = FontProject::new();
        let ids = [("acute", 1), ("A", 2)].into_iter().collect();
        let source = "anchorDef <300 700> TOP_ANCHOR; markClass acute <anchor TOP_ANCHOR> @TOP; feature mark { pos base A <anchor 300 700> mark @TOP; } mark;";
        let expanded = expand_named_anchors(source);
        assert!(expanded.contains("<anchor 300 700>"));
        assert!(build_kerning_gpos(&project, &ids, source).is_some());
    }
