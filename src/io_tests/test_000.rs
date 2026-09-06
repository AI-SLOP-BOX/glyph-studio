    #[test]
    fn project_json_round_trip_preserves_font_data() {
        let path = std::env::temp_dir().join(format!("glyph-studio-{}.json", std::process::id()));
        let mut project = FontProject::new();
        project.metadata.x_height = 500.0;
        project.metadata.cap_height = 700.0;
        project.metadata.family_name = "Round Trip".into();
        project.guidelines.push(crate::font_data::Guideline {
            x: 10.0,
            y: 700.0,
            angle: 15.0,
            name: "global".into(),
        });
        let mut glyph = GlyphData::new("A".into(), Some(65));
        glyph.guidelines.push(crate::font_data::Guideline {
            x: 20.0,
            y: 300.0,
            angle: 90.0,
            name: "glyph".into(),
        });
        glyph.unicodes = vec![0xFF21];
        glyph.left_kerning_group = "A-group".into();
        glyph.right_kerning_group = "V-group".into();
        glyph.left_kerning_group = "A-group".into();
        glyph.right_kerning_group = "V-group".into();
        glyph.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
            ],
        });
        glyph.components.push(GlyphComponent {
            base: "acute".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 10.0,
            y_offset: 20.0,
        });
        project.glyphs.insert("A".into(), glyph);
        project.glyph_order.push("A".into());
        project.color_palettes = vec![vec![[255, 0, 0, 255], [0, 0, 255, 255]]];
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A".into(),
                palette_index: 0,
                alpha: 1.0,
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
                    stops: Vec::new(),
                    radius0: 0.0,
                    radius1: 500.0,
                    start_angle: 0.0,
                    end_angle: 360.0,
                }),
            }],
        );
        project.normalize_masters();
        save_project(&project, &path).unwrap();
        let loaded = load_project(&path).unwrap();
        assert_eq!(loaded, project);
        assert_eq!(loaded.metadata.x_height, 500.0);
        assert_eq!(loaded.metadata.cap_height, 700.0);
        std::fs::remove_file(path).unwrap();
    }
