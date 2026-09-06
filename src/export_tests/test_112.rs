    #[test]
    fn feature_validation_reports_statement_line_without_an_offset() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.opentype_features = "feature liga { sub missing by A; } liga;".into();
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("OpenType feature 1行目") && issue.contains("missing")));
    }
