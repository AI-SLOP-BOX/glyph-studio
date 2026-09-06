    #[test]
    fn fit_widths_to_outlines_updates_every_master_layer() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.width = 900.0;
        glyph.contours.push(Contour {
            points: vec![ContourPoint::on_curve(430.0, 20.0)],
        });
        glyph.layers.insert(
            "bold".into(),
            GlyphLayer {
                width: 1100.0,
                contours: vec![Contour {
                    points: vec![ContourPoint::on_curve(520.0, 20.0)],
                }],
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);

        assert_eq!(project.fit_widths_to_outlines(&["A".into()]), 1);
        assert_eq!(project.glyphs["A"].width, 430.0);
        assert_eq!(project.glyphs["A"].layers["bold"].width, 520.0);
    }
