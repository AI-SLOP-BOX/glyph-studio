    #[test]
    fn reverse_contour_updates_authored_geometry_and_all_layers() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour.clone()];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![contour.clone()],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.reverse_contour_all_layers(0).unwrap();
        let expected: Vec<_> = contour.points.into_iter().rev().collect();
        assert_eq!(glyph.contours[0].points, expected);
        assert_eq!(glyph.layers["regular"].contours[0].points, expected);
    }
