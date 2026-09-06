    #[test]
    fn master_svg_export_rejects_unknown_master_without_mutating_project() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let original = project.clone();
        let directory =
            std::env::temp_dir().join(format!("glyph-studio-svg-missing-{}", std::process::id()));
        let result = export_all_svg_for_master(&project, "missing", &directory);
        assert!(result.is_err());
        assert_eq!(project, original);
        assert!(!directory.exists());
    }
