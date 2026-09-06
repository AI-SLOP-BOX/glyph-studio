    #[test]
    fn exports_ttf_with_mixed_contours_and_components() {
        let mut project = FontProject::new();
        project.add_glyph("base".into(), Some('A' as u32));
        project.add_glyph("mixed".into(), Some('B' as u32));
        project
            .glyphs
            .get_mut("base")
            .unwrap()
            .contours
            .push(Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(100.0, 100.0),
                ],
            });
        project
            .glyphs
            .get_mut("mixed")
            .unwrap()
            .contours
            .push(Contour {
                points: vec![
                    ContourPoint::on_curve(200.0, 0.0),
                    ContourPoint::on_curve(300.0, 0.0),
                    ContourPoint::on_curve(300.0, 100.0),
                ],
            });
        project.glyphs.get_mut("mixed").unwrap().components.push(
            crate::font_data::GlyphComponent {
                base: "base".into(),
                x_scale: 1.0,
                xy_scale: 0.0,
                yx_scale: 0.0,
                y_scale: 1.0,
                x_offset: 0.0,
                y_offset: 0.0,
            },
        );
        let path =
            std::env::temp_dir().join(format!("glyph-studio-mixed-{}.ttf", std::process::id()));
        export_ttf(&project, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert!(ttf_parser::Face::parse(&bytes, 0).is_ok());
        std::fs::remove_file(path).unwrap();
    }
