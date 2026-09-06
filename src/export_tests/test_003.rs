    #[test]
    fn exported_ttf_round_trips_unmodelled_opentype_tables() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        let payload = vec![0, 1, 2, 3, 4, 5];
        project
            .preserved_tables
            .insert("MATH".into(), payload.clone());
        let base_payload = vec![0, 1, 2, 3];
        project
            .preserved_tables
            .insert("BASE".into(), base_payload.clone());
        let colr_payload = vec![0, 1, 0, 0, 0, 0];
        project
            .preserved_tables
            .insert("COLR".into(), colr_payload.clone());
        let path = std::env::temp_dir().join(format!(
            "glyph-studio-preserved-table-{}.ttf",
            std::process::id()
        ));
        export_ttf(&project, &path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let font = read_fonts::FontRef::new(&bytes).unwrap();
        assert_eq!(
            font.table_data(Tag::new(b"MATH")).unwrap().as_bytes(),
            payload
        );
        assert_eq!(
            font.table_data(Tag::new(b"BASE")).unwrap().as_bytes(),
            base_payload
        );
        assert_eq!(
            font.table_data(Tag::new(b"COLR")).unwrap().as_bytes(),
            colr_payload
        );
        std::fs::remove_file(path).unwrap();
    }
