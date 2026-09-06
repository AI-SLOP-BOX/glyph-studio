    #[test]
    fn normalize_glyph_winding_preserves_alternating_counter_direction() {
        let mut project = FontProject::new();
        project.add_glyph("O".into(), Some('O' as u32));
        let glyph = project.glyphs.get_mut("O").unwrap();
        glyph.contours = vec![
            Contour {
                points: vec![
                    ContourPoint::on_curve(0.0, 0.0),
                    ContourPoint::on_curve(100.0, 0.0),
                    ContourPoint::on_curve(100.0, 100.0),
                    ContourPoint::on_curve(0.0, 100.0),
                ],
            },
            Contour {
                points: vec![
                    ContourPoint::on_curve(25.0, 25.0),
                    ContourPoint::on_curve(25.0, 75.0),
                    ContourPoint::on_curve(75.0, 75.0),
                    ContourPoint::on_curve(75.0, 25.0),
                ],
            },
        ];
        project.normalize_glyph_winding(&["O".into()]);
        let contours = &project.glyphs["O"].contours;
        assert!(contours[0].signed_area() < 0.0);
        assert!(contours[1].signed_area() > 0.0);
    }
