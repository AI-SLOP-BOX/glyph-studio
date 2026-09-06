    #[test]
    fn feature_table_overrides_apply_to_vmtx_glyph_metrics() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some(65));
        glyph.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 100.0),
            ],
        });
        project.glyphs.insert("A".into(), glyph);
        apply_feature_table_overrides(
            &mut project,
            "table vmtx { VertOriginY A 800; VertAdvanceY A 1200; } vmtx;",
        );
        let metric = project.vertical_metrics["A"];
        assert_eq!(metric.top_side_bearing, 700.0);
        assert_eq!(metric.advance_height, 1200.0);
    }
