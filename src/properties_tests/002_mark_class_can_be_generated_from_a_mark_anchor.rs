    #[test]
    fn mark_class_can_be_generated_from_a_mark_anchor() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("acute".to_string(), Some(0x00B4));
        glyph.anchors.push(GlyphAnchor {
            name: "_top".to_string(),
            x: 12.4,
            y: 503.6,
        });
        project.glyphs.insert(glyph.name.clone(), glyph);

        assert_eq!(
            ensure_mark_class_for_glyph(&mut project, "acute"),
            Some("@top".to_string())
        );
        assert!(project
            .opentype_features
            .contains("markClass acute <anchor 12 504> @top;"));
    }
