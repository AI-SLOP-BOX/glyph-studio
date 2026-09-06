    #[test]
    fn project_validation_reports_invalid_conditional_layers() {
        let mut project = FontProject::new();
        project.conditional_layers.insert(
            "Missing".into(),
            vec![crate::font_data::ConditionalLayer {
                id: "alt".into(),
                conditions: std::collections::HashMap::from([(
                    "wght".into(),
                    crate::font_data::AxisRange {
                        min: Some(700.0),
                        max: Some(400.0),
                    },
                )]),
                layer: GlyphLayer {
                    width: 600.0,
                    contours: Vec::new(),
                    components: Vec::new(),
                    anchors: Vec::new(),
                },
            }],
        );
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("条件レイヤーが存在しない")));
        assert!(issues.iter().any(|issue| issue.contains("軸範囲が不正")));
    }
