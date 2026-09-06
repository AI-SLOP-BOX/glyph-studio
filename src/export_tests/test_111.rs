    #[test]
    fn feature_validation_handles_multiline_statements() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.opentype_features = "feature liga {\n  sub A\n    by missing;\n} liga;".into();
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("missing")));
    }
