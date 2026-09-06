    #[test]
    fn cursive_anchors_compile_into_gpos() {
        let mut project = FontProject::new();
        for (name, entry, exit) in [
            ("alef", (0.0, 500.0), (500.0, 500.0)),
            ("beh", (0.0, 500.0), (500.0, 500.0)),
        ] {
            let mut glyph = GlyphData::new(name.into(), None);
            glyph.anchors.extend([
                GlyphAnchor {
                    name: "entry".into(),
                    x: entry.0,
                    y: entry.1,
                },
                GlyphAnchor {
                    name: "exit".into(),
                    x: exit.0,
                    y: exit.1,
                },
            ]);
            project.glyphs.insert(name.into(), glyph);
        }
        let ids = [("alef", 1), ("beh", 2)].into_iter().collect();
        let bytes = build_kerning_gpos(&project, &ids, "").unwrap();
        assert!(bytes.len() > 40);
    }
