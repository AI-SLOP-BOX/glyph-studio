    #[test]
    fn colr_v1_gradient_and_transform_round_trip_through_ttf() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("A.color".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 255], [0, 0, 255, 255]]];
        project.color_palette_names = vec!["Brand Light".into()];
        project.color_palette_types = vec![0x0000_0001];
        project.color_palette_entry_names = vec!["Fill".into(), "Outline".into()];
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.color".into(),
                palette_index: 0,
                gradient: Some(crate::font_data::ColorGradient {
                    start_palette_index: 0,
                    end_palette_index: 1,
                    kind: crate::font_data::ColorGradientKind::Linear,
                    extend: crate::font_data::ColorGradientExtend::Reflect,
                    x0: 0.0,
                    y0: 0.0,
                    x1: 500.0,
                    y1: 0.0,
                    x2: 0.0,
                    y2: 500.0,
                    stops: vec![
                        crate::font_data::ColorGradientStop {
                            offset: 0.0,
                            palette_index: 0,
                            alpha: 1.0,
                        },
                        crate::font_data::ColorGradientStop {
                            offset: 1.0,
                            palette_index: 1,
                            alpha: 0.75,
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
            vec![Some(crate::font_data::ColorLayerTransform {
                xx: 1.1,
                yx: 0.0,
                xy: 0.0,
                yy: 0.9,
                dx: 12.0,
                dy: -8.0,
            })],
        );
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-colr-v1-roundtrip-{}.ttf",
            std::process::id()
        ));
        export_ttf(&project, &path).unwrap();
        let exported_bytes = std::fs::read(&path).unwrap();
        let exported_font = read_fonts::FontRef::new(&exported_bytes).unwrap();
        let exported_cpal = exported_font.cpal().unwrap();
        assert_eq!(exported_cpal.version(), 1);
        let exported_labels = exported_cpal.palette_labels_array().unwrap().unwrap();
        assert_eq!(exported_labels[0].get().to_u16(), 1000);
        let exported_name = exported_font.name().unwrap();
        let exported_name_data = exported_name.string_data();
        assert!(exported_name
            .name_record()
            .iter()
            .any(|record| record.name_id().to_u16() == 1000
                && record.string(exported_name_data).is_ok()));
        assert_eq!(
            exported_name
                .name_record()
                .iter()
                .find(|record| record.name_id().to_u16() == 1000)
                .unwrap()
                .string(exported_name_data)
                .unwrap()
                .chars()
                .collect::<String>(),
            "Brand Light"
        );
        let loaded = crate::io::load_ttf(&path).unwrap();
        let layer = &loaded.color_layers["A"][0];
        let gradient = layer.gradient.as_ref().unwrap();
        assert_eq!(layer.glyph, "A.color");
        assert_eq!(gradient.kind, crate::font_data::ColorGradientKind::Linear);
        assert_eq!(
            gradient.extend,
            crate::font_data::ColorGradientExtend::Reflect
        );
        assert_eq!(gradient.stops.len(), 2);
        assert!((gradient.stops[1].alpha - 0.75).abs() < 0.01);
        let transform = loaded.color_layer_transforms["A"][0].unwrap();
        assert!((transform.xx - 1.1).abs() < 0.001);
        assert!((transform.dy + 8.0).abs() < 0.001);
        assert_eq!(loaded.color_palette_names, vec!["Brand Light"]);
        assert_eq!(loaded.color_palette_types, vec![0x0000_0001]);
        assert_eq!(loaded.color_palette_entry_names, vec!["Fill", "Outline"]);
        std::fs::remove_file(path).unwrap();
    }
