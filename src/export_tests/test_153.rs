    #[test]
    fn project_validation_reports_nested_color_cycles() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.color_palettes = vec![vec![[255, 0, 0, 255]]];
        project.color_layers.insert(
            "A".into(),
            vec![crate::font_data::ColorLayer {
                glyph: "A".into(),
                palette_index: 0,
                gradient: None,
                alpha: 1.0,
            }],
        );
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("COLRカラーグリフ循環参照")));
    }
