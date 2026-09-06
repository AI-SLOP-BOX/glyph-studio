    #[test]
    fn exported_ttf_resolves_bmp_and_supplementary_unicode_together() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.add_glyph("grinning".into(), Some('😀' as u32));
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-mixed-unicode-{}.ttf",
            std::process::id()
        ));
        export_ttf(&project, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let face = ttf_parser::Face::parse(&bytes, 0).unwrap();
        assert!(face.glyph_index('A').is_some());
        assert!(face.glyph_index('😀').is_some());
        std::fs::remove_file(path).unwrap();
    }
