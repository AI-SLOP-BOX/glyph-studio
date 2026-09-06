    #[test]
    fn set_width_for_glyphs_updates_existing_names_only() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("B".into(), Some('B' as u32));
        project.glyphs.get_mut("A").unwrap().layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let names = vec!["A".into(), "missing".into(), "B".into()];
        assert_eq!(project.set_width_for_glyphs(&names, 720.0), 2);
        assert_eq!(project.glyphs["A"].width, 720.0);
        assert_eq!(project.glyphs["A"].layers["regular"].width, 720.0);
        assert_eq!(project.glyphs["B"].width, 720.0);
        assert_eq!(project.set_width_for_glyphs(&names, -1.0), 0);
    }
