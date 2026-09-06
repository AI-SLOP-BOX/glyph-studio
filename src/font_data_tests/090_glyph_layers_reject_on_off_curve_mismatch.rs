    #[test]
    fn glyph_layers_reject_on_off_curve_mismatch() {
        let a = GlyphLayer {
            width: 500.0,
            contours: vec![Contour {
                points: vec![ContourPoint::on_curve(0.0, 0.0)],
            }],
            components: Vec::new(),
            anchors: Vec::new(),
        };
        let b = GlyphLayer {
            width: 500.0,
            contours: vec![Contour {
                points: vec![ContourPoint::off_curve(10.0, 10.0)],
            }],
            components: Vec::new(),
            anchors: Vec::new(),
        };
        assert!(a.interpolate(&b, 0.5).is_none());
    }
