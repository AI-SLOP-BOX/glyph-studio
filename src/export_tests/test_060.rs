    #[test]
    fn project_validation_rejects_duplicate_and_unknown_glyph_order_entries() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.glyph_order = vec!["A".into(), "A".into(), "missing".into()];
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("グリフ順序に重複")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("グリフ順序に未定義")));
    }
