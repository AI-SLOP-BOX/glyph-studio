    #[test]
    fn set_smooth_nodes_rejects_layer_structure_mismatch_atomically() {
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
                contours: vec![Contour { points: vec![] }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let before = glyph.clone();
        assert!(glyph.set_smooth_nodes_all_layers(&[(0, 0)], true).is_err());
        assert_eq!(glyph, before);
    }
