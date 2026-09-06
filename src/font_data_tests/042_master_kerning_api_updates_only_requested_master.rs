    #[test]
    fn master_kerning_api_updates_only_requested_master() {
        let mut project = FontProject::new();
        project.add_glyph("A".into(), Some(65));
        project.add_glyph("V".into(), Some(86));
        project.masters.push(FontMaster {
            id: "bold".into(),
            name: "Bold".into(),
            weight: 700.0,
            width: 100.0,
            is_bracket: false,
            axes: HashMap::new(),
        });
        project
            .set_kerning_pair_for_master("bold", "A", "V", -100.0)
            .unwrap();
        assert!(!project.kerning.contains_key(&("A".into(), "V".into())));
        assert_eq!(
            project.kerning_by_master["bold"][&("A".into(), "V".into())],
            -100.0
        );
        assert!(project
            .set_kerning_pair_for_master("missing", "A", "V", -20.0)
            .is_err());
    }
