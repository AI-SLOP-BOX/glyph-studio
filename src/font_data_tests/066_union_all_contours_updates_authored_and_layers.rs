    #[test]
    fn union_all_contours_updates_authored_and_layers() {
        let rectangle = |x: f64| Contour {
            points: vec![
                ContourPoint::on_curve(x, 0.0),
                ContourPoint::on_curve(x + 100.0, 0.0),
                ContourPoint::on_curve(x + 100.0, 100.0),
                ContourPoint::on_curve(x, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![rectangle(0.0), rectangle(50.0), rectangle(1000.0)];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: glyph.contours.clone(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.union_all_contours_all_layers().unwrap();
        assert_eq!(glyph.contours.len(), 2);
        assert_eq!(glyph.layers["regular"].contours.len(), 2);
    }
