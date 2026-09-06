    #[test]
    fn glyph_layers_interpolate_bilinear_uses_both_axes() {
        let base = GlyphLayer {
            width: 500.0,
            contours: vec![Contour {
                points: vec![ContourPoint::on_curve(0.0, 0.0)],
            }],
            components: Vec::new(),
            anchors: Vec::new(),
        };
        let mut right = base.clone();
        right.width = 700.0;
        let mut top = base.clone();
        top.width = 900.0;
        let mut top_right = base.clone();
        top_right.width = 1100.0;
        let middle = base
            .interpolate_bilinear(&right, &top, &top_right, 0.5, 0.5)
            .unwrap();
        assert_eq!(middle.width, 800.0);
    }
