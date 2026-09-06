    #[test]
    fn exports_each_master_as_a_separate_static_otf() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "太字".into(),
            ..FontMaster::default()
        });
        let directory =
            std::env::temp_dir().join(format!("glyph-studio-otf-masters-{}", std::process::id()));
        let count = export_all_otf_for_masters(&project, &directory).unwrap();
        assert_eq!(count, project.masters.len());
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), count);
        for entry in std::fs::read_dir(&directory).unwrap() {
            let bytes = std::fs::read(entry.unwrap().path()).unwrap();
            assert_eq!(&bytes[..4], b"OTTO");
        }
        std::fs::remove_dir_all(directory).unwrap();
    }
