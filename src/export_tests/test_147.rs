    #[test]
    fn color_tables_encode_colr_layers_and_cpal_bgra_records() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("A.red".into(), None);
        project.add_glyph("A.green".into(), None);
        project.add_glyph("A.blue".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 128], [0, 32, 255, 255]]];
        project.color_layers.insert(
            "A".into(),
            vec![
                crate::font_data::ColorLayer {
                    glyph: "A.red".into(),
                    palette_index: 1,
                    gradient: Some(crate::font_data::ColorGradient {
                        start_palette_index: 0,
                        end_palette_index: 1,
                        kind: crate::font_data::ColorGradientKind::Linear,
                        extend: crate::font_data::ColorGradientExtend::default(),
                        x0: 0.0,
                        y0: 0.0,
                        x1: 1000.0,
                        y1: 0.0,
                        x2: 0.0,
                        y2: 1000.0,
                        stops: vec![
                            crate::font_data::ColorGradientStop {
                                offset: 0.0,
                                palette_index: 0,
                                alpha: 1.0,
                            },
                            crate::font_data::ColorGradientStop {
                                offset: 0.5,
                                palette_index: 1,
                                alpha: 0.5,
                            },
                            crate::font_data::ColorGradientStop {
                                offset: 1.0,
                                palette_index: 0,
                                alpha: 1.0,
                            },
                        ],
                        radius0: 0.0,
                        radius1: 500.0,
                        start_angle: 0.0,
                        end_angle: 360.0,
                    }),
                    alpha: 1.0,
                },
                crate::font_data::ColorLayer {
                    glyph: "A.green".into(),
                    palette_index: 0,
                    gradient: Some(crate::font_data::ColorGradient {
                        start_palette_index: 0,
                        end_palette_index: 1,
                        kind: crate::font_data::ColorGradientKind::Radial,
                        extend: crate::font_data::ColorGradientExtend::default(),
                        x0: 100.0,
                        y0: 200.0,
                        x1: 300.0,
                        y1: 400.0,
                        x2: 300.0,
                        y2: 200.0,
                        stops: Vec::new(),
                        radius0: 10.0,
                        radius1: 500.0,
                        start_angle: 0.0,
                        end_angle: 360.0,
                    }),
                    alpha: 1.0,
                },
                crate::font_data::ColorLayer {
                    glyph: "A.blue".into(),
                    palette_index: 0,
                    gradient: Some(crate::font_data::ColorGradient {
                        start_palette_index: 0,
                        end_palette_index: 1,
                        kind: crate::font_data::ColorGradientKind::Sweep,
                        extend: crate::font_data::ColorGradientExtend::default(),
                        x0: 500.0,
                        y0: 500.0,
                        x1: 0.0,
                        y1: 0.0,
                        x2: 1000.0,
                        y2: 500.0,
                        stops: Vec::new(),
                        radius0: 0.0,
                        radius1: 500.0,
                        start_angle: 30.0,
                        end_angle: 270.0,
                    }),
                    alpha: 1.0,
                },
            ],
        );
        project.color_layer_transforms.insert(
            "A".into(),
            vec![Some(crate::font_data::ColorLayerTransform {
                xx: 0.9,
                yx: 0.1,
                xy: -0.1,
                yy: 1.1,
                dx: 24.0,
                dy: -12.0,
            })],
        );
        let ids = [("A", 1), ("A.red", 2), ("A.green", 3), ("A.blue", 4)]
            .into_iter()
            .collect();
        let (colr, cpal) = build_color_tables(&project, &ids).unwrap();
        assert_eq!(&colr[0..2], &[0, 1]);
        assert_eq!(u16::from_be_bytes([colr[2], colr[3]]), 1);
        assert_eq!(u16::from_be_bytes([colr[12], colr[13]]), 3);
        assert_ne!(
            u32::from_be_bytes([colr[14], colr[15], colr[16], colr[17]]),
            0
        );
        assert_eq!(&cpal[0..2], &[0, 0]);
        assert_eq!(u16::from_be_bytes([cpal[2], cpal[3]]), 2);
        assert_eq!(&cpal[20..24], &[255, 32, 0, 255]);
        assert!(colr
            .windows(7)
            .any(|window| window == [10, 0, 0, 6, 0, 2, 4]));
        assert!(colr
            .windows(7)
            .any(|window| window == [10, 0, 0, 6, 0, 3, 6]));
        assert!(colr
            .windows(7)
            .any(|window| window == [10, 0, 0, 6, 0, 4, 8]));
        assert!(colr
            .windows(7)
            .any(|window| window == [12, 0, 0, 31, 0, 0, 7]));
    }
