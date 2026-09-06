    #[test]
    fn project_validation_reports_missing_default_master() {
        let mut project = FontProject::new();
        project.default_master_id = "missing".into();
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("デフォルトマスター")));
    }
