    #[test]
    fn project_validation_rejects_duplicate_anchor_names() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), None);
        glyph.anchors = vec![
            GlyphAnchor {
                name: "top".into(),
                x: 0.0,
                y: 700.0,
            },
            GlyphAnchor {
                name: " top ".into(),
                x: 10.0,
                y: 700.0,
            },
        ];
        project.glyphs.insert("A".into(), glyph);
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("アンカー名 'top' が重複")));
    }
