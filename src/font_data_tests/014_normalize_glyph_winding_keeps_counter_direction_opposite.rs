    #[test]
    fn normalize_glyph_winding_keeps_counter_direction_opposite() {
        let triangle = |offset: f64, size: f64| Contour {
            points: vec![
                ContourPoint::on_curve(offset, offset),
                ContourPoint::on_curve(offset + size, offset),
                ContourPoint::on_curve(offset, offset + size),
            ],
        };
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![triangle(0.0, 200.0), triangle(50.0, 50.0)];
        project.glyphs.insert("A".into(), glyph);
        assert_eq!(project.normalize_glyph_winding(&["A".into()]), 1);
        let contours = &project.glyphs["A"].contours;
        assert!(contours[0].signed_area() * contours[1].signed_area() < 0.0);
    }
