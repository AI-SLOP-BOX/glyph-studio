    #[test]
    fn project_validation_reports_invalid_color_layers() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.color_palettes = vec![vec![[0, 0, 0, 255]]];
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "missing".into(),
                palette_index: 4,
                gradient: None,
                alpha: 1.0,
            }],
        );
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("未定義グリフ 'missing'")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("パレット番号が範囲外")));
    }
