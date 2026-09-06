    #[test]
    fn component_ligatures_emit_gdef_caret_list() {
        let mut project = FontProject::new();
        project.add_glyph("f".into(), None);
        project.add_glyph("i".into(), None);
        let mut ligature = GlyphData::new("f_i".into(), None);
        ligature.components = vec![
            GlyphComponent {
                base: "f".into(),
                x_scale: 1.0,
                xy_scale: 0.0,
                yx_scale: 0.0,
                y_scale: 1.0,
                x_offset: 0.0,
                y_offset: 0.0,
            },
            GlyphComponent {
                base: "i".into(),
                x_scale: 1.0,
                xy_scale: 0.0,
                yx_scale: 0.0,
                y_scale: 1.0,
                x_offset: 0.0,
                y_offset: 0.0,
            },
        ];
        project.glyphs.insert("f_i".into(), ligature);
        let ids = [("f", 1), ("i", 2), ("f_i", 3)].into_iter().collect();
        assert!(build_gdef(&project, &ids, "").is_some());
    }
