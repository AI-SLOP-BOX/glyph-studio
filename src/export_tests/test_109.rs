    #[test]
    fn feature_validation_reports_undefined_named_classes() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.opentype_features = "feature calt { sub @missing A' by A; } calt;".into();
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("未定義クラス")));
    }
