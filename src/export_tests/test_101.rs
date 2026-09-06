    #[test]
    fn project_validation_rejects_out_of_range_kerning_values() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.kerning.insert(("A".into(), "A".into()), 40000.0);
        assert!(validate_project(&project)
            .iter()
            .any(|issue| issue.contains("カーニング値が範囲外")));
    }
