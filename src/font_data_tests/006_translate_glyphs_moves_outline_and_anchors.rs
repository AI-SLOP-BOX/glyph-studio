    #[test]
    fn translate_glyphs_moves_outline_and_anchors() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.contours.push(Contour {
            points: vec![ContourPoint::on_curve(10.0, 20.0)],
        });
        glyph.anchors.push(GlyphAnchor {
            name: "top".into(),
            x: 5.0,
            y: 6.0,
        });
        project.glyphs.insert("A".into(), glyph);
        assert_eq!(project.translate_glyphs(&["A".into()], 12.0, -3.0), 1);
        assert_eq!(project.glyphs["A"].contours[0].points[0].x, 22.0);
        assert_eq!(project.glyphs["A"].anchors[0].y, 3.0);
    }
