    #[test]
    fn svg_export_preserves_bezier_commands() {
        let mut project = FontProject::new();
        project.metadata.ascender = 900.0;
        project.metadata.descender = -250.0;
        let mut glyph = GlyphData::new("curve".into(), None);
        glyph.width = 720.0;
        glyph.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(50.0, 100.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, -100.0),
                ContourPoint::on_curve(0.0, -100.0),
            ],
        });
        project.glyphs.insert("curve".into(), glyph);
        let path =
            std::env::temp_dir().join(format!("glyph-studio-curve-{}.svg", std::process::id()));
        export_svg(&project, "curve", &path).unwrap();
        let svg = std::fs::read_to_string(&path).unwrap();
        assert!(svg.contains("Q "));
        assert!(svg.contains("fill-rule"));
        assert!(svg.contains("viewBox=\"0 -900 720 1150\""));
        let mut base = GlyphData::new("base".into(), None);
        base.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(40.0, 0.0),
                ContourPoint::on_curve(40.0, 40.0),
            ],
        });
        project.glyphs.insert("base".into(), base);
        let mut composite = GlyphData::new("composite".into(), None);
        composite.components.push(GlyphComponent {
            base: "base".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: 1.0,
            x_offset: 100.0,
            y_offset: 25.0,
        });
        project.glyphs.insert("composite".into(), composite);
        export_svg(&project, "composite", &path).unwrap();
        let composite_svg = std::fs::read_to_string(&path).unwrap();
        assert_eq!(composite_svg.matches("<path").count(), 1);
        assert!(composite_svg.contains("100"));
        std::fs::remove_file(path).unwrap();
    }
