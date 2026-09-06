    #[test]
    fn knife_split_is_applied_to_all_layers_atomically() {
        let contour = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
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
        glyph.split_segment_all_layers(0, 0, 0.5).unwrap();
        glyph.split_segment_all_layers(0, 2, 0.5).unwrap();
        glyph.cut_contour_all_layers(0, 1, 3).unwrap();
        assert_eq!(glyph.contours.len(), 2);
        assert_eq!(glyph.layers["bold"].contours.len(), 2);
    }
