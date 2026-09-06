    #[test]
    fn feature_source_base_positioning_accepts_multiple_mark_anchors() {
        let project = FontProject::new();
        let ids = [("A", 1), ("acute", 2), ("grave", 3)].into_iter().collect();
        let source = "markClass acute <anchor 0 0> @top; markClass grave <anchor 0 0> @bottom; feature mark { pos base A <anchor 300 700> mark @top <anchor 300 0> mark @bottom; } mark;";
        let bytes = build_kerning_gpos(&project, &ids, source).unwrap();
        assert!(bytes.len() > 40);
    }
