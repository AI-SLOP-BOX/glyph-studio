    #[test]
    fn masters_normalize_empty_and_duplicate_ids() {
        let mut project = FontProject::new();
        project.masters = vec![
            FontMaster {
                id: "regular".into(),
                name: "Regular".into(),
                weight: 400.0,
                width: 100.0,
                is_bracket: false,
                axes: [("wght".into(), 400.0)].into_iter().collect(),
            },
            FontMaster {
                id: "regular".into(),
                name: "Duplicate".into(),
                ..FontMaster::default()
            },
            FontMaster {
                id: " ".into(),
                name: "Invalid".into(),
                ..FontMaster::default()
            },
        ];
        project.normalize_masters();
        assert_eq!(project.masters.len(), 1);
        assert_eq!(project.masters[0].id, "regular");
    }
