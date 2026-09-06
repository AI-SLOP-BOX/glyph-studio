    #[test]
    fn align_and_distribute_nodes_sync_all_layers() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 10.0),
                ContourPoint::on_curve(40.0, 30.0),
                ContourPoint::on_curve(100.0, 50.0),
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
            .align_nodes_all_layers(&[(0, 0), (0, 1)], true)
            .unwrap();
        glyph
            .distribute_nodes_all_layers(&[(0, 0), (0, 1), (0, 2)], true)
            .unwrap();
        assert_eq!(glyph.contours[0].points[0].x, 0.0);
        assert_eq!(glyph.contours[0].points[1].x, 50.0);
        assert_eq!(glyph.layers["bold"].contours[0].points[1].x, 50.0);
    }
