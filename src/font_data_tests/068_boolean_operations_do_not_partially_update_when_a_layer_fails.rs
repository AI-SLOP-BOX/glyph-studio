    #[test]
    fn boolean_operations_do_not_partially_update_when_a_layer_fails() {
        let rectangle = |x: f64| Contour {
            points: vec![
                ContourPoint::on_curve(x, 0.0),
                ContourPoint::on_curve(x + 100.0, 0.0),
                ContourPoint::on_curve(x + 100.0, 100.0),
                ContourPoint::on_curve(x, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![rectangle(0.0), rectangle(50.0)];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: vec![rectangle(0.0)],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let before = glyph.contours.clone();
        assert!(glyph.union_contours_all_layers(0).is_err());
        assert_eq!(glyph.contours, before);
    }
