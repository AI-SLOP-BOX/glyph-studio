    #[test]
    fn remove_contour_updates_authored_geometry_and_all_layers() {
        let contour = |x: f64| Contour {
            points: vec![
                ContourPoint::on_curve(x, 0.0),
                ContourPoint::on_curve(x + 10.0, 0.0),
                ContourPoint::on_curve(x, 10.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour(0.0), contour(100.0)];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: glyph.width,
                contours: glyph.contours.clone(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.remove_contour_all_layers(0).unwrap();
        assert_eq!(glyph.contours.len(), 1);
        assert_eq!(glyph.layers["regular"].contours.len(), 1);
        assert_eq!(glyph.contours[0].points[0].x, 100.0);
    }
