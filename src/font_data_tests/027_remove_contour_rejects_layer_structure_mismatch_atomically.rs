    #[test]
    fn remove_contour_rejects_layer_structure_mismatch_atomically() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(10.0, 0.0),
                ContourPoint::on_curve(0.0, 10.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![contour.clone()];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: glyph.width,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let before = glyph.clone();
        assert!(glyph.remove_contour_all_layers(0).is_err());
        assert_eq!(glyph, before);
    }
