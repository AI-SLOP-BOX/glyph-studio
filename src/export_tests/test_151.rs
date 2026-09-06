    #[test]
    fn svg_export_expands_nested_color_glyphs() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("A.inner".into(), None);
        project.add_glyph("A.leaf".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 255], [0, 0, 255, 255]]];
        project.color_layers.insert(
            "A.inner".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.leaf".into(),
                palette_index: 1,
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
        project
            .glyphs
            .get_mut("A.leaf")
            .unwrap()
            .contours
            .push(Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(0.0, 100.0),
                ],
            });
        let svg = build_svg_document(&project, "A").unwrap();
        assert!(svg.contains("fill=\"none\" fill-opacity=\"1.000000\""));
        assert!(svg.contains("fill=\"#0000ff\""));
        assert_eq!(svg.matches("<path").count(), 1);
    }
