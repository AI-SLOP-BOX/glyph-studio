    #[test]
    fn project_validation_reports_orphaned_background_references() {
        let mut project = FontProject::new();
        project
            .background_images
            .entry("Missing".into())
            .or_default()
            .insert("unknown-master".into(), "/tmp/ref.png".into());
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("存在しないグリフ")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("存在しないマスター")));
    }
