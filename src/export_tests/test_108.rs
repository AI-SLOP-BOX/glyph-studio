    #[test]
    fn feature_validation_reports_unknown_glyph_references() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("B".into(), Some('B' as u32));
        project.opentype_features = "feature liga { sub A by missing; } liga;".into();
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("missing")));
    }
