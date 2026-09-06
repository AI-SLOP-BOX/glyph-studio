    #[test]
    fn project_validation_reports_invalid_variation_sequences() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.unicode_variation_sequences = vec![
            UnicodeVariationSequence {
                base: 0xD800,
                selector: 0xFE00,
                glyph: "missing".into(),
            },
            UnicodeVariationSequence {
                base: 0x4E00,
                selector: 0xFE00,
                glyph: "A".into(),
            },
            UnicodeVariationSequence {
                base: 0x4E00,
                selector: 0xFE00,
                glyph: "A".into(),
            },
        ];
        let issues = validate_project(&project);
        assert!(issues
            .iter()
            .any(|issue| issue.contains("IVSのUnicodeまたはセレクタ")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("存在しないグリフ")));
        assert!(issues
            .iter()
            .any(|issue| issue.contains("IVSのUnicode／セレクタが重複")));
    }
