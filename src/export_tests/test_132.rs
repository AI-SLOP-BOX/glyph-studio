    #[test]
    fn exports_each_master_as_a_separate_woff2() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Regular".into(),
            ..FontMaster::default()
        });
        let directory =
            std::env::temp_dir().join(format!("glyph-studio-woff2-masters-{}", std::process::id()));
        let count = export_all_woff2_for_masters(&project, &directory).unwrap();
        assert_eq!(count, project.masters.len());
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), count);
        for entry in std::fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("woff2"));
            assert_eq!(&std::fs::read(&path).unwrap()[..4], b"wOF2");
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
