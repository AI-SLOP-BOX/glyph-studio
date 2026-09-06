    #[test]
    fn transform_nodes_syncs_each_master_around_its_own_selection_center() {
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
            .transform_nodes_all_layers(&[(0, 0), (0, 1)], 2.0, 0.0)
            .unwrap();
        assert_eq!(glyph.contours[0].points[0].x, -50.0);
        assert_eq!(glyph.layers["bold"].contours[0].points[1].x, 150.0);
    }
