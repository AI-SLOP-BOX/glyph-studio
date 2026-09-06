    #[test]
    fn svg_export_encodes_color_gradients_and_spread_method() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("A.red".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 255], [0, 255, 0, 255]]];
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.red".into(),
                palette_index: 0,
                gradient: Some(crate::font_data::ColorGradient {
                    start_palette_index: 0,
                    end_palette_index: 1,
                    kind: crate::font_data::ColorGradientKind::Linear,
                    extend: crate::font_data::ColorGradientExtend::Reflect,
                    x0: 0.0,
                    y0: 0.0,
                    x1: 100.0,
                    y1: 0.0,
                    x2: 0.0,
                    y2: 100.0,
                    stops: vec![
                        crate::font_data::ColorGradientStop {
                            offset: 0.0,
                            palette_index: 0,
                            alpha: 1.0,
                        },
                        crate::font_data::ColorGradientStop {
                            offset: 0.5,
                            palette_index: 1,
                            alpha: 0.75,
                        },
                    ],
                    radius0: 0.0,
                    radius1: 100.0,
                    start_angle: 0.0,
                    end_angle: 360.0,
                }),
                alpha: 1.0,
            }],
        );
        project
            .glyphs
            .get_mut("A.red")
            .unwrap()
            .contours
            .push(Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(0.0, 100.0),
                ],
            });
        project.color_layer_transforms.insert(
            "A".into(),
            vec![Some(crate::font_data::ColorLayerTransform {
                xx: 1.0,
                yx: 0.0,
                xy: 0.0,
                yy: 1.0,
                dx: 12.0,
                dy: -6.0,
            })],
        );
        let svg = build_svg_document(&project, "A").unwrap();
        assert!(svg.contains("<linearGradient"));
        assert!(svg.contains("spreadMethod=\"reflect\""));
        assert!(svg.contains("matrix(1 0 0 1 12 -6)"));
        assert_eq!(svg.matches("<stop ").count(), 2);
    }
