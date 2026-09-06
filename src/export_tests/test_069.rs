    #[test]
    fn mark_to_ligature_anchors_compile_into_gpos() {
        let mut project = FontProject::new();
        let mut mark = GlyphData::new("acute".into(), None);
        mark.anchors.push(GlyphAnchor {
            name: "_top".into(),
            x: 0.0,
            y: 0.0,
        });
        let mut ligature = GlyphData::new("f_i".into(), None);
        ligature.anchors.extend([
            GlyphAnchor {
                name: "top_1".into(),
                x: 250.0,
                y: 700.0,
            },
            GlyphAnchor {
                name: "top_2".into(),
                x: 550.0,
                y: 700.0,
            },
        ]);
        project.glyphs.insert("acute".into(), mark);
        project.glyphs.insert("f_i".into(), ligature);
        let ids = [("acute", 1), ("f_i", 2)].into_iter().collect();
        let bytes = build_kerning_gpos(&project, &ids, "").unwrap();
        assert!(bytes.len() > 40);
    }
