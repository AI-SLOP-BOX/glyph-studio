    #[test]
    fn preserved_layout_table_is_used_when_no_source_replacement_exists() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let payload = vec![0, 1, 0, 0, 0, 0, 0, 0];
        project
            .preserved_tables
            .insert("GSUB".into(), payload.clone());
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-preserved-gsub-{}.ttf",
            std::process::id()
        ));
        export_ttf(&project, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let font = read_fonts::FontRef::new(&bytes).unwrap();
        assert_eq!(
            font.table_data(Tag::new(b"GSUB")).unwrap().as_bytes(),
            payload
        );
        std::fs::remove_file(path).unwrap();
    }
