    #[test]
    fn normalize_contour_directions_updates_every_layer() {
        let clockwise = Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
            ],
        };
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.contours = vec![clockwise.clone()];
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: glyph.width,
                contours: vec![clockwise],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let mut project = FontProject::new();
        project.glyphs.insert(glyph.name.clone(), glyph);
        assert_eq!(project.normalize_glyph_winding(&["A".into()]), 1);
        let glyph = &project.glyphs["A"];
        assert!(glyph.contours[0].signed_area() <= 0.0);
        assert!(glyph.layers["regular"].contours[0].signed_area() <= 0.0);
    }
