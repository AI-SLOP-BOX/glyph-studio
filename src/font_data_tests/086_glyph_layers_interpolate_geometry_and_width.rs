    #[test]
    fn glyph_layers_interpolate_geometry_and_width() {
        let a = GlyphLayer {
            width: 500.0,
            contours: vec![Contour {
                points: vec![ContourPoint::on_curve(0.0, 0.0)],
            }],
            components: Vec::new(),
            anchors: vec![GlyphAnchor {
                name: "top".into(),
                x: 100.0,
                y: 200.0,
            }],
        };
        let b = GlyphLayer {
            width: 700.0,
            contours: vec![Contour {
                points: vec![ContourPoint::on_curve(100.0, 200.0)],
            }],
            components: Vec::new(),
            anchors: vec![GlyphAnchor {
                name: "top".into(),
                x: 300.0,
                y: 400.0,
            }],
        };
        let middle = a.interpolate(&b, 0.5).unwrap();
        assert_eq!(middle.width, 600.0);
        assert_eq!(middle.contours[0].points[0].x, 50.0);
        assert_eq!(middle.contours[0].points[0].y, 100.0);
        assert_eq!(middle.anchors.len(), 1);
        assert_eq!(middle.anchors[0].x, 200.0);
        assert_eq!(middle.anchors[0].y, 300.0);
    }
