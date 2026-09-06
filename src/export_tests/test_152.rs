    #[test]
    fn svg_export_preserves_nested_color_gradients() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("A.inner".into(), None);
        project.add_glyph("A.leaf".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 255], [0, 0, 255, 255]]];
        project.color_layers.insert(
            "A.inner".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.leaf".into(),
                palette_index: 0,
                gradient: Some(crate::font_data::ColorGradient {
                    start_palette_index: 0,
                    end_palette_index: 1,
                    kind: crate::font_data::ColorGradientKind::Linear,
                    extend: crate::font_data::ColorGradientExtend::Pad,
                    x0: 0.0,
                    y0: 0.0,
                    x1: 100.0,
                    y1: 0.0,
                    x2: 0.0,
                    y2: 100.0,
                    stops: Vec::new(),
                    radius0: 0.0,
                    radius1: 100.0,
                    start_angle: 0.0,
                    end_angle: 360.0,
                }),
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
        assert!(svg.contains("id=\"glyph-studio-nested-gradient-0-0\""));
        assert!(svg.contains("fill=\"url(#glyph-studio-nested-gradient-0-0)\""));
        assert!(svg
            .contains("fill=\"url(#glyph-studio-nested-gradient-0-0)\" fill-opacity=\"1.000000\""));
        assert_eq!(svg.matches("<stop ").count(), 2);
    }
