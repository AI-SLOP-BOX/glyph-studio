    #[test]
    fn feature_class_validation_reports_duplicates_and_missing_glyphs() {
        let mut glyphs = std::collections::HashMap::new();
        glyphs.insert("A".into(), GlyphData::new("A".into(), Some('A' as u32)));
        let issues =
            validate_feature_class_definitions("@Upper = [A Missing]; @Upper = [A];", &glyphs);
        assert!(issues.iter().any(|issue| issue.contains("重複")));
        assert!(issues.iter().any(|issue| issue.contains("Missing")));
    }
