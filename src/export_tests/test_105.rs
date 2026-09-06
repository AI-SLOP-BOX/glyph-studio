    #[test]
    fn project_validation_reports_orphaned_master_layers() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.layers.insert(
            "deleted-master".into(),
            GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("未定義マスター")));
    }
