    #[test]
    fn exports_each_master_as_a_separate_woff() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Regular".into(),
            ..FontMaster::default()
        });
        let directory =
            std::env::temp_dir().join(format!("glyph-studio-woff-masters-{}", std::process::id()));
        let count = export_all_woff_for_masters(&project, &directory).unwrap();
        assert_eq!(count, 2);
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 2);
        std::fs::remove_dir_all(directory).unwrap();
    }
