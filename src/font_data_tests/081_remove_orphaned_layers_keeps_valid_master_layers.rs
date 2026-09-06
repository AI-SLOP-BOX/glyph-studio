    #[test]
    fn remove_orphaned_layers_keeps_valid_master_layers() {
        let mut project = FontProject::new();
        let mut glyph = GlyphData::new("A".into(), Some('A' as u32));
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        glyph.layers.insert(
            "deleted".into(),
            GlyphLayer {
                width: 700.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("A".into(), glyph);
        assert_eq!(project.remove_orphaned_layers(), 1);
        assert!(project.glyphs["A"].layers.contains_key("regular"));
        assert!(!project.glyphs["A"].layers.contains_key("deleted"));
    }
