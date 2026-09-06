    #[test]
    fn layout_fingerprint_ignores_outlines_but_tracks_layout_inputs() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        let original = layout_input_fingerprint(&project);
        project.glyphs.get_mut("A").unwrap().contours.push(Contour {
            points: vec![
                ContourPoint::on_curve(0.0, 0.0),
                ContourPoint::on_curve(100.0, 0.0),
                ContourPoint::on_curve(0.0, 100.0),
            ],
        });
        assert_eq!(layout_input_fingerprint(&project), original);
        project
            .glyphs
            .get_mut("A")
            .unwrap()
            .anchors
            .push(GlyphAnchor {
                name: "top".into(),
                x: 50.0,
                y: 700.0,
            });
        assert_ne!(layout_input_fingerprint(&project), original);
    }
