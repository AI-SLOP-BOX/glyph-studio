    #[test]
    fn feature_validation_checks_glyphs_inside_classes() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.opentype_features = "feature liga { sub [A absent] by A; } liga;".into();
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("absent")));
    }
