    #[test]
    fn toggle_curve_nodes_ignores_unselected_invalid_contours() {
        let valid = Contour {
            points: vec![
                ContourPoint::off_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let invalid = Contour {
            points: vec![ContourPoint::on_curve(10.0, 10.0)],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![valid, invalid];
        glyph.toggle_curve_nodes_all_layers(&[(0, 0)]).unwrap();
        assert!(glyph.contours[0].points[0].is_on_curve());
        assert_eq!(glyph.contours[1].points.len(), 1);
    }
