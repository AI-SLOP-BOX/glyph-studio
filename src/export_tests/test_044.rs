    #[test]
    fn vorg_contains_non_default_vertical_origins() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let glyph = project.glyphs.get_mut("A").unwrap();
        glyph.contours = vec![Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(100.0, 900.0),
            ],
        }];
        let vorg = build_vorg(&project, "regular").unwrap();
        assert_eq!(&vorg[0..4], &[0, 1, 0, 0]);
        assert_eq!(u16::from_be_bytes([vorg[6], vorg[7]]), 1);
        assert_eq!(u16::from_be_bytes([vorg[8], vorg[9]]), 1);
    }
