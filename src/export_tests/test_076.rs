    #[test]
    fn feature_source_mark_to_ligature_preserves_null_component_slots() {
        let mut project = FontProject::new();
        project.add_glyph("acute".into(), None);
        project.add_glyph("f_i".into(), None);
        let ids = [("acute", 1), ("f_i", 2)].into_iter().collect();
        let source = "markClass acute <anchor 0 0> @top; feature mark { pos ligature f_i NULL <anchor 550 700> mark @top; } mark;";
        let bytes = build_kerning_gpos(&project, &ids, source).unwrap();
        assert!(bytes.len() > 40);
    }
