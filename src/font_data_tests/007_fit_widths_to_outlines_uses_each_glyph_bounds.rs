    #[test]
    fn fit_widths_to_outlines_uses_each_glyph_bounds() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.width = 900.0;
        glyph.contours.push(Contour {
            points: vec![ContourPoint::on_curve(430.0, 20.0)],
        });
        project.glyphs.insert("A".into(), glyph);
        assert_eq!(project.fit_widths_to_outlines(&["A".into()]), 1);
        assert_eq!(project.glyphs["A"].width, 430.0);
    }
