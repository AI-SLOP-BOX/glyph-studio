    #[test]
    fn project_validation_rejects_invalid_background_opacity() {
        let mut project = FontProject::new();
        project
            .background_opacities
            .entry("A".into())
            .or_default()
            .insert("regular".into(), 1.5);
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("背景画像不透明度")));
    }
