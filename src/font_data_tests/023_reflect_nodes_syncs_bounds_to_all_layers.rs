    #[test]
    fn reflect_nodes_syncs_bounds_to_all_layers() {
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
            "bold".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![contour],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph
            .reflect_nodes_all_layers(&[(0, 0), (0, 1)], true)
            .unwrap();
        assert_eq!(glyph.contours[0].points[0].x, 100.0);
        assert_eq!(glyph.layers["bold"].contours[0].points[1].x, 0.0);
    }
