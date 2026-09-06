    #[test]
    fn removing_palette_color_updates_layer_and_gradient_references() {
        let mut project = FontProject::new();
        project.color_palettes = vec![vec![[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]]];
        project.color_layers.insert(
            "A".into(),
            vec![ColorLayer {
                glyph: "A.red".into(),
                palette_index: 2,
                gradient: Some(ColorGradient {
                    start_palette_index: 2,
                    end_palette_index: 1,
                    kind: ColorGradientKind::Linear,
                    extend: Default::default(),
                    x0: 0.0,
                    y0: 0.0,
                    x1: 100.0,
                    y1: 0.0,
                    x2: 0.0,
                    y2: 100.0,
                    stops: vec![
                        ColorGradientStop {
                            offset: 0.0,
                            palette_index: 2,
                            alpha: 1.0,
                        },
                        ColorGradientStop {
                            offset: 1.0,
                            palette_index: 1,
                            alpha: 1.0,
                        },
                    ],
                    radius0: 0.0,
                    radius1: 0.0,
                    start_angle: 0.0,
                    end_angle: 360.0,
                }),
                alpha: 1.0,
            }],
        );
        project.color_layer_transforms.insert(
            "A".into(),
            vec![Some(ColorLayerTransform {
                dx: 10.0,
                ..ColorLayerTransform::default()
            })],
        );

        remove_color_palette_entry(&mut project, 1);

        assert_eq!(project.color_palettes[0].len(), 2);
        let layer = &project.color_layers["A"][0];
        assert_eq!(layer.palette_index, 1);
        let gradient = layer.gradient.as_ref().unwrap();
        assert_eq!(gradient.start_palette_index, 1);
        assert_eq!(gradient.end_palette_index, 0);
        assert_eq!(gradient.stops.len(), 1);
        assert_eq!(gradient.stops[0].palette_index, 1);
        assert_eq!(project.color_layer_transforms["A"][0].unwrap().dx, 10.0);
    }
