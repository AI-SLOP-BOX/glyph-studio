    #[test]
    fn project_validation_reports_unicode_noncharacters() {
        let mut project = FontProject::new();
        project.add_glyph("noncharacter".into(), Some(0xFDD0));
        let issues = validate_project(&project);
        assert!(issues.iter().any(|issue| issue.contains("非文字")));
    }
