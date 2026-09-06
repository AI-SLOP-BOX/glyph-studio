    #[test]
    fn toggle_curve_nodes_updates_authored_geometry_and_all_layers() {
        let point = ContourPoint::off_curve(0.0, 0.0);
        let contour = Contour {
            points: vec![
                point,
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
                contours: vec![contour],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.toggle_curve_nodes_all_layers(&[(0, 0)]).unwrap();
        assert!(glyph.contours[0].points[0].is_on_curve());
        assert!(glyph.layers["regular"].contours[0].points[0].is_on_curve());
    }
