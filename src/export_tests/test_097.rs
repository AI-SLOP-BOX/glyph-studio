    #[test]
    fn variable_widths_emit_hvar() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let mut second = project.masters[0].clone();
        second.id = "bold".into();
        second.name = "Bold".into();
        second.weight = 700.0;
        project.masters.push(second.clone());
        project.glyphs.get_mut("A").unwrap().width = 500.0;
        project.glyphs.get_mut("A").unwrap().layers.insert(
            second.id.clone(),
            GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        let bytes = build_hvar(&project, &["A"], &project.masters[0], &["wght".into()]);
        assert!(bytes.is_some());
    }
