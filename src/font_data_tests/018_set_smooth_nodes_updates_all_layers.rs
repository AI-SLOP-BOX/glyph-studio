    #[test]
    fn set_smooth_nodes_updates_all_layers() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::off_curve(50.0, 100.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour.clone()];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![contour],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.set_smooth_nodes_all_layers(&[(0, 0)], true).unwrap();
        assert!(glyph.contours[0].points[0].smooth);
        assert!(glyph.layers["regular"].contours[0].points[0].smooth);
    }
