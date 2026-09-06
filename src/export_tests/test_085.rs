    #[test]
    fn feature_table_name_records_are_written_to_ttf() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.opentype_features = "table name { nameid 256 \"Display Name\"; } name;".to_string();
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-name-table-{}.ttf",
            std::process::id()
        ));
        export_ttf(&project, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let font = read_fonts::FontRef::new(&bytes).unwrap();
        let names = font.name().unwrap();
        assert!(names
            .name_record()
            .iter()
            .any(|record| record.name_id().to_u16() == 256));
        std::fs::remove_file(path).unwrap();
    }
