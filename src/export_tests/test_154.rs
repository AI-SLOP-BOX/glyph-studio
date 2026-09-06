    #[test]
    fn svg_export_encodes_color_layers_with_palette_alpha() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("A.red".into(), None);
        project.color_palettes = vec![vec![[255, 0, 0, 128]], vec![[0, 255, 0, 255]]];
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A.red".into(),
                palette_index: 0,
                gradient: None,
                alpha: 0.5,
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
        let path =
            std::env::temp_dir().join(format!("glyph-studio-color-{}.svg", std::process::id()));
        export_svg_with_palette(&project, "A", 1, &path).unwrap();
        let svg = std::fs::read_to_string(&path).unwrap();
        assert!(svg.contains("fill=\"#00ff00\""));
        assert!(svg.contains("fill-opacity=\"0.500000\""));
        assert_eq!(svg.matches("<path").count(), 1);
        std::fs::remove_file(path).unwrap();
    }
