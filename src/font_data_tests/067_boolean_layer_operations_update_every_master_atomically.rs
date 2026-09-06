    #[test]
    fn boolean_layer_operations_update_every_master_atomically() {
        let rectangle = |x: f64| Contour {
            points: vec![
                ContourPoint::on_curve(x, 0.0),
                ContourPoint::on_curve(x + 100.0, 0.0),
                ContourPoint::on_curve(x + 100.0, 100.0),
                ContourPoint::on_curve(x, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![rectangle(0.0), rectangle(50.0)];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: glyph.contours.clone(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.union_contours_all_layers(0).unwrap();
        assert_eq!(glyph.contours.len(), glyph.layers["regular"].contours.len());
        let mut difference_glyph = glyph.clone();
        difference_glyph.contours = vec![rectangle(0.0), rectangle(50.0)];
        difference_glyph.layers.get_mut("regular").unwrap().contours =
            difference_glyph.contours.clone();
        difference_glyph.difference_contours_all_layers(0).unwrap();
        assert_eq!(
            difference_glyph.contours.len(),
            difference_glyph.layers["regular"].contours.len()
        );
        let mut intersection_glyph = GlyphData::new("intersection".into(), None);
        intersection_glyph.contours = vec![rectangle(0.0), rectangle(50.0)];
        intersection_glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: intersection_glyph.contours.clone(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        intersection_glyph
            .intersection_contours_all_layers(0)
            .unwrap();
        assert_eq!(
            intersection_glyph.contours.len(),
            intersection_glyph.layers["regular"].contours.len()
        );
        let mut xor_glyph = GlyphData::new("xor".into(), None);
        xor_glyph.contours = vec![rectangle(0.0), rectangle(50.0)];
        xor_glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: xor_glyph.contours.clone(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        xor_glyph.xor_contours_all_layers(0).unwrap();
        assert_eq!(
            xor_glyph.contours.len(),
            xor_glyph.layers["regular"].contours.len()
        );
    }
