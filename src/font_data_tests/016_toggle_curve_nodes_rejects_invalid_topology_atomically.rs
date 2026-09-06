    #[test]
    fn toggle_curve_nodes_rejects_invalid_topology_atomically() {
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
        let before = glyph.clone();
        assert!(glyph.toggle_curve_nodes_all_layers(&[(0, 0)]).is_err());
        assert_eq!(glyph, before);
    }
