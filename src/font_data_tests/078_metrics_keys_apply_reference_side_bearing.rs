    #[test]
    fn metrics_keys_apply_reference_side_bearing() {
        let mut project = FontProject::new();
        let mut reference = GlyphData::new("H".into(), Some('H' as u32));
        reference.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(50.0, 0.0),
                ContourPoint::on_curve(300.0, 0.0),
                ContourPoint::on_curve(300.0, 700.0),
            ],
        });
        project.glyphs.insert("H".into(), reference);
        let mut target = GlyphData::new("A".into(), Some('A' as u32));
        target.contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(200.0, 0.0),
                ContourPoint::on_curve(200.0, 700.0),
            ],
        });
        target.left_metrics_key = "=H".into();
        project.glyphs.insert("A".into(), target);
        assert_eq!(project.apply_metrics_keys(&["A".into()]).unwrap(), 1);
        let glyph = &project.glyphs["A"];
        assert_eq!(glyph.contours[0].points[0].x, 50.0);
        assert_eq!(glyph.width, 550.0);
        assert!(project.apply_metrics_keys(&["missing".into()]).is_err());
    }
