    #[test]
    fn exports_all_glyphs_to_safe_svg_filenames() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("あ".into(), Some('あ' as u32));
        project.add_glyph("い".into(), Some('い' as u32));
        let directory =
            std::env::temp_dir().join(format!("glyph-studio-svg-{}", std::process::id()));
        let count = export_all_svg(&project, &directory).unwrap();
        assert_eq!(count, 3);
        assert!(directory.join("A.svg").is_file());
        assert!(directory.join("_.svg").is_file());
        assert!(directory.join("__2.svg").is_file());
        std::fs::remove_dir_all(directory).unwrap();
    }
