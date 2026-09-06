    #[test]
    fn project_validation_reports_invalid_axis_display_names() {
        let mut project = FontProject::new();
        project.masters[0].axes.insert("wght".into(), 400.0);
        project.axis_names.insert("wght".into(), "".into());
        project.axis_names.insert("wdth".into(), "Weight".into());
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("表示名が空")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("存在しない軸タグ")));
    }
