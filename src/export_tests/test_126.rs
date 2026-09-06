    #[test]
    fn interpolation_rejects_glyphs_missing_a_master_layer() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.glyphs.get_mut("A").unwrap().layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 600.0,
                contours: Vec::new(),
                components: Vec::new(),
                anchors: Vec::new(),
            },
        );
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            ..FontMaster::default()
        });
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-missing-layer-{}.ttf",
            std::process::id()
        ));
        let error = export_ttf_at_interpolation(&project, "regular", "bold", 0.5, &path)
            .expect_err("missing master layer must be rejected");
        assert!(error.contains("補間元マスター") || error.contains("補間先マスター"));
        assert!(!path.exists());
    }
