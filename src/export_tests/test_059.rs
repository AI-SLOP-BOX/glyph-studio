    #[test]
    fn project_validation_rejects_invalid_names_master_ids_and_layer_transforms() {
        let mut project = FontProject::new();
        project.masters.push(FontMaster {
            id: project.masters[0].id.clone(),
            ..FontMaster::default()
        });
        let mut glyph = GlyphData::new("different".into(), None);
        glyph.components.push(GlyphComponent {
            base: "different".into(),
            x_scale: 1.0,
            xy_scale: 0.0,
            yx_scale: 0.0,
            y_scale: f64::INFINITY,
            x_offset: 0.0,
            y_offset: 0.0,
        });
        glyph.layers.insert(
            "regular".into(),
            GlyphLayer {
                width: 500.0,
                contours: Vec::new(),
                components: vec![GlyphComponent {
                    base: "different".into(),
                    x_scale: f64::NAN,
                    xy_scale: 0.0,
                    yx_scale: 0.0,
                    y_scale: 1.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                }],
                anchors: Vec::new(),
            },
        );
        project.glyphs.insert("bad name".into(), glyph);
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("グリフ名が不正")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("グリフ 'bad name' のコンポーネント変換")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("マスターIDが重複")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("グリフ名の登録が不一致")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("コンポーネント変換が不正")));
    }
