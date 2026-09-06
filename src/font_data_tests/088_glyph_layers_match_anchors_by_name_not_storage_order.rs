    #[test]
    fn glyph_layers_match_anchors_by_name_not_storage_order() {
        let layer = |anchors: Vec<GlyphAnchor>| GlyphLayer {
            width: 500.0,
            contours: vec![Contour {
                points: vec![ContourPoint::on_curve(0.0, 0.0)],
            }],
            components: Vec::new(),
            anchors,
        };
        let a = layer(vec![
            GlyphAnchor {
                name: "top".into(),
                x: 0.0,
                y: 100.0,
            },
            GlyphAnchor {
                name: "bottom".into(),
                x: 0.0,
                y: -100.0,
            },
        ]);
        let b = layer(vec![
            GlyphAnchor {
                name: "bottom".into(),
                x: 20.0,
                y: -80.0,
            },
            GlyphAnchor {
                name: "top".into(),
                x: 20.0,
                y: 120.0,
            },
        ]);
        let middle = a.interpolate(&b, 0.5).unwrap();
        assert_eq!(middle.anchors[0].name, "top");
        assert_eq!(middle.anchors[0].x, 10.0);
        assert_eq!(middle.anchors[1].name, "bottom");
    }
