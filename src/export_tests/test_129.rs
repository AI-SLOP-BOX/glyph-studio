    #[test]
    fn exports_each_master_as_a_separate_static_ttf() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Regular".into(),
            ..FontMaster::default()
        });
        let directory =
            std::env::temp_dir().join(format!("glyph-studio-masters-{}", std::process::id()));
        let count = export_all_ttf_for_masters(&project, &directory).unwrap();
        assert_eq!(count, project.masters.len());
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), count);
        std::fs::remove_dir_all(directory).unwrap();
    }
