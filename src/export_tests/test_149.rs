    #[test]
    fn color_tables_keep_base_and_layer_order_stable_for_hash_maps() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("B".into(), Some(66));
        project.add_glyph("A.layer".into(), None);
        project.add_glyph("B.layer".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 255]]];
        project.color_layers.insert(
            "B".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "B.layer".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.layer".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        let ids = [("A", 1), ("B", 2), ("A.layer", 3), ("B.layer", 4)]
            .into_iter()
            .collect();
        let (colr, _) = build_color_tables(&project, &ids).unwrap();
        assert_eq!(&colr[34..40], &[0, 1, 0, 0, 0, 1]);
        assert_eq!(&colr[40..46], &[0, 2, 0, 1, 0, 1]);
        assert_eq!(&colr[46..50], &[0, 3, 0, 0]);
        assert_eq!(&colr[50..54], &[0, 4, 0, 0]);
    }
