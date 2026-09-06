    #[test]
    fn project_validation_rejects_invalid_font_metadata() {
        let mut project = FontProject::new();
        project.metadata.family_name.clear();
        project.metadata.style_name = "   ".into();
        project.metadata.units_per_em = 0.0;
        project.metadata.ascender = f64::NAN;
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("ファミリー名が空")));
        assert!(issues.iter().any(|issue| issue.contains("スタイル名が空")));
        assert!(issues.iter().any(|issue| issue.contains("UPMが")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("フォントメトリクス")));
    }
