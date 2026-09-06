    #[test]
    fn exports_requested_interpolation_set() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some('A' as u32));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            ..FontMaster::default()
        });
        let directory =
            std::env::temp_dir().join(format!("glyph-studio-set-{}", std::process::id()));
        let count =
            export_interpolation_set(&project, "regular", "bold", &[0.1, 0.5, 0.9], &directory)
                .unwrap();
        assert_eq!(count, 3);
        assert!(directory.join("instance-10.ttf").exists());
        assert!(directory.join("instance-50.ttf").exists());
        assert!(directory.join("instance-90.ttf").exists());
        assert!(export_interpolation_set(&project, "regular", "bold", &[], &directory).is_err());
        assert!(
            export_interpolation_set(&project, "regular", "bold", &[0.5, 0.5], &directory).is_err()
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
