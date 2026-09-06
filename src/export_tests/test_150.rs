    #[test]
    fn color_tables_can_reuse_nested_color_glyphs() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("A.inner".into(), None);
        project.add_glyph("A.leaf".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 255]]];
        project.color_layers.insert(
            "A.inner".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.leaf".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.inner".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        let ids = [("A", 1), ("A.inner", 2), ("A.leaf", 3)]
            .into_iter()
            .collect();
        let (colr, _) = build_color_tables(&project, &ids).unwrap();
        assert!(colr.windows(3).any(|window| window == [11, 0, 2]));
    }
